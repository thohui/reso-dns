use futures::StreamExt;
use std::{sync::Arc, time::Duration};

use arc_swap::ArcSwap;
use reso_dns::domain_name::DomainName;
use reso_list::DomainListMatcher;
use tokio::{
    sync::Mutex,
    time::{self, MissedTickBehavior},
};

use crate::{
    database::{
        CoreDatabasePool, DatabaseError,
        models::{ListAction, domain_rule::DomainRule, list_subscription::ListSubscription},
    },
    global::SharedGlobal,
    utils::uuid::EntityId,
};

use super::ServiceError;

/// Validate and normalize a domain pattern.
/// Returns the canonical stored form, e.g. `*.example.com` or `example.com`.
fn normalize_domain_pattern(input: &str) -> Result<String, ServiceError> {
    // what we are doing here is kinda hacky, since DomainName doesn't support wildcard patterns, but we want to reuse its validation and normalization logic.
    let (wildcard, base) = match input.strip_prefix("*.") {
        Some(rest) => (true, rest),
        None => (false, input),
    };
    if base.contains('*') {
        return Err(ServiceError::BadRequest(
            "Wildcards are only supported as a prefix (e.g. *.example.com)".into(),
        ));
    }
    let name = DomainName::from_user(base).map_err(|e| ServiceError::BadRequest(format!("Invalid domain: {e}")))?;
    Ok(if wildcard {
        format!("*.{name}")
    } else {
        name.to_string()
    })
}

pub struct Matchers {
    pub blocklist_matcher: Arc<DomainListMatcher>,
    pub allow_list_matcher: Arc<DomainListMatcher>,
}

impl Matchers {
    /// Load the matchers from db.
    pub async fn load(db: &CoreDatabasePool) -> anyhow::Result<Self> {
        let allow_list = DomainRule::list_enabled_by_action(ListAction::Allow, db).await?;
        let block_list = DomainRule::list_enabled_by_action(ListAction::Block, db).await?;
        Ok(Self {
            blocklist_matcher: Arc::new(DomainListMatcher::load(
                block_list.iter().filter(|d| d.enabled).map(|d| d.domain.as_str()),
            )?),
            allow_list_matcher: Arc::new(DomainListMatcher::load(
                allow_list.iter().filter(|d| d.enabled).map(|d| d.domain.as_str()),
            )?),
        })
    }
}

const SUBSCRIPTION_SYNC_INTERVAL_SECS: u64 = 60 * 60 * 24; // 24 hours
const SUBSCRIPTION_FETCH_TIMEOUT_SECS: u64 = 50;
const SUBSCRIPTION_MAX_RESPONSE_BYTES: u64 = 50 * 1024 * 1024; // 50 MB

pub struct DomainRulesService {
    matchers: ArcSwap<Matchers>,
    write_lock: Mutex<()>,
    connection: Arc<CoreDatabasePool>,
    http_client: reqwest::Client,
}

impl DomainRulesService {
    /// Initialize a `DomainRulesService` instance.
    pub async fn initialize(connection: Arc<CoreDatabasePool>) -> anyhow::Result<Self> {
        Ok(Self {
            matchers: ArcSwap::new(Matchers::load(&connection).await?.into()),
            write_lock: Mutex::new(()),
            connection,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(SUBSCRIPTION_FETCH_TIMEOUT_SECS))
                .build()?,
        })
    }

    /// Add a new domain rule with the given domain pattern and action.
    pub async fn add_domain(&self, domain: &str, action: ListAction) -> Result<(), ServiceError> {
        let domain = normalize_domain_pattern(domain)?;

        let mut rule = DomainRule::new(domain);
        rule.action = action;

        rule.insert(&self.connection).await.map_err(|e| {
            if e.is_unique_constraint_violation() {
                ServiceError::Conflict("Domain already has a rule".into())
            } else {
                ServiceError::Internal(e.into())
            }
        })?;

        match action {
            ListAction::Allow => self.reload_allow_list().await?,
            ListAction::Block => self.reload_blocklist().await?,
        }

        Ok(())
    }

    /// Remove a domain rule by domain pattern.
    pub async fn remove_domain(&self, domain: &str) -> Result<(), ServiceError> {
        let domain = normalize_domain_pattern(domain)?;

        let changed = DomainRule::delete_by_domain(&domain, &self.connection).await?;

        if !changed {
            return Err(ServiceError::NotFound("Domain not found".into()));
        }

        self.reload_all().await?;
        Ok(())
    }

    /// Update the action of an existing domain rule.
    pub async fn update_domain_action(&self, domain: &str, action: ListAction) -> Result<(), ServiceError> {
        let domain = normalize_domain_pattern(domain)?;

        let changed = DomainRule::update_action(&domain, action, &self.connection).await?;

        if !changed {
            return Err(ServiceError::NotFound("Domain not found".into()));
        }

        self.reload_all().await?;
        Ok(())
    }

    /// Toggle the enabled state of an individual domain rule.
    pub async fn toggle_domain(&self, domain: &str) -> Result<(), ServiceError> {
        let domain = normalize_domain_pattern(domain)?;

        let changed = DomainRule::toggle(&domain, &self.connection).await?;

        if !changed {
            return Err(ServiceError::NotFound("Domain not found".into()));
        }

        self.reload_all().await?;

        Ok(())
    }

    /// Reload both the blocklist and allowlist.
    async fn reload_all(&self) -> Result<(), ServiceError> {
        let _guard = self.write_lock.lock().await;

        self.matchers.swap(
            Matchers::load(&self.connection)
                .await
                .map_err(ServiceError::Internal)?
                .into(),
        );
        Ok(())
    }

    /// Reload the allow list
    async fn reload_allow_list(&self) -> Result<(), ServiceError> {
        let _guard = self.write_lock.lock().await;

        let rules = DomainRule::list_enabled_by_action(ListAction::Allow, &self.connection).await?;

        let new_matcher =
            Arc::new(DomainListMatcher::load(rules.iter().map(|r| r.domain.as_str())).map_err(ServiceError::Internal)?);

        self.matchers.rcu(|current| {
            Arc::new(Matchers {
                allow_list_matcher: Arc::clone(&new_matcher),
                blocklist_matcher: Arc::clone(&current.blocklist_matcher),
            })
        });

        Ok(())
    }

    /// Reload the blocklist.
    async fn reload_blocklist(&self) -> Result<(), ServiceError> {
        let _guard = self.write_lock.lock().await;
        let rules = DomainRule::list_enabled_by_action(ListAction::Block, &self.connection).await?;
        let new_matcher =
            Arc::new(DomainListMatcher::load(rules.iter().map(|r| r.domain.as_str())).map_err(ServiceError::Internal)?);

        self.matchers.rcu(|current| {
            Arc::new(Matchers {
                blocklist_matcher: Arc::clone(&new_matcher),
                allow_list_matcher: Arc::clone(&current.allow_list_matcher),
            })
        });

        Ok(())
    }
    /// Check if a given domain name is blocked by the matcher.
    pub fn is_blocked(&self, name: &str) -> bool {
        let matchers = self.matchers.load();
        if matchers.blocklist_matcher.exists(name) {
            return !matchers.allow_list_matcher.exists(name);
        }
        false
    }

    /// List all subscriptions with their current domain counts (derived from domain_rules).
    pub async fn list_subscriptions_with_counts(&self) -> Result<Vec<(ListSubscription, i64)>, ServiceError> {
        Ok(ListSubscription::list_with_domain_counts(&self.connection).await?)
    }

    /// Remove a list subscription by ID.
    pub async fn remove_list_subscription(&self, id: EntityId<ListSubscription>) -> Result<(), ServiceError> {
        let changed = ListSubscription::delete_by_id(id, &self.connection).await?;
        if !changed {
            return Err(ServiceError::NotFound("Subscription not found".into()));
        }
        self.reload_all().await?;
        Ok(())
    }

    /// Toggle the enabled state of a list subscription by ID.
    /// This also causes all the underlying domain rules from the subscription to be toggled to the same value.
    pub async fn toggle_list_subscription(&self, id: EntityId<ListSubscription>) -> Result<(), ServiceError> {
        let changed = ListSubscription::toggle_enabled(id, &self.connection).await?;
        if !changed {
            return Err(ServiceError::NotFound("Subscription not found".into()));
        }
        self.reload_all().await?;
        Ok(())
    }

    /// Toggles the sync_enabled state of a list subscription by ID.
    pub async fn toggle_list_subscription_sync_enabled(
        &self,
        id: EntityId<ListSubscription>,
    ) -> Result<(), ServiceError> {
        let changed = ListSubscription::toggle_sync_enabled(id, &self.connection).await?;
        if !changed {
            return Err(ServiceError::NotFound("Subscription not found".into()));
        }
        Ok(())
    }

    /// Sync ALL list subscriptions, fetching updated rules from their URLs if needed.
    pub async fn sync_subscriptions(&self) {
        let subscriptions = match ListSubscription::list(&self.connection).await {
            Ok(subs) => subs,
            Err(e) => {
                tracing::error!("failed to load list subscriptions: {}", e);
                return;
            }
        };

        let mut any_updated = false;

        for sub in subscriptions.iter().filter(|s| s.enabled && s.sync_enabled) {
            match fetch_domain_rules_from_list_subscription_task(sub, &self.http_client, &self.connection).await {
                Ok(updated) => {
                    if updated {
                        any_updated = true;
                    }
                }
                Err(e) => {
                    tracing::error!("failed to fetch domain rules from subscription {}: {}", sub.url, e);
                }
            }
        }

        if !any_updated {
            tracing::info!("no list subscriptions were updated during sync");
            return;
        }

        if let Err(e) = self.reload_all().await {
            tracing::error!("failed to reload matchers after subscription sync: {}", e);
        }
    }

    /// Add a new list subscription with the given URL and list type.
    /// This will also trigger an immediate sync for the new subscription.
    pub async fn add_list_subscription(&self, list_subscription: ListSubscription) -> Result<(), ServiceError> {
        validate_list_subscription_url(&list_subscription.url)?;

        // send a head request first to check if the url is reachable.
        let head_response = self
            .http_client
            .head(&list_subscription.url)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() || e.is_timeout() {
                    ServiceError::BadRequest("URL is not reachable".into())
                } else if e.status() == Some(reqwest::StatusCode::NOT_FOUND) {
                    ServiceError::BadRequest("URL not found".into())
                } else {
                    ServiceError::Internal(e.into())
                }
            })?;

        head_response.error_for_status_ref().map_err(|e| {
            if e.is_connect() || e.is_timeout() {
                ServiceError::BadRequest("URL is not reachable".into())
            } else if e.status() == Some(reqwest::StatusCode::NOT_FOUND) {
                ServiceError::BadRequest("URL not found".into())
            } else {
                ServiceError::Internal(e.into())
            }
        })?;

        let content_type = head_response.headers().get("content-type");

        if let Some(content_type) = content_type
            && !content_type
                .to_str()
                .map_err(|_| ServiceError::BadRequest("Invalid content-type header from URL".into()))?
                .contains("text/plain")
        {
            return Err(ServiceError::BadRequest(
                "URL must have content-type of text/plain".into(),
            ));
        }

        list_subscription.clone().insert(&self.connection).await.map_err(|e| {
            if e.is_unique_constraint_violation() {
                ServiceError::Conflict("A subscription with the same URL already exists".into())
            } else {
                ServiceError::Internal(e.into())
            }
        })?;

        // run initial sync so the user gets rules immediately.
        if let Err(e) =
            fetch_domain_rules_from_list_subscription_task(&list_subscription, &self.http_client, &self.connection)
                .await
        {
            // rollback
            if let Err(e) = ListSubscription::delete_by_id(list_subscription.id, &self.connection).await {
                tracing::error!("failed to delete list subscription after failing sync: {}", e);
            }
            tracing::error!("failed to fetch domain rules from subscription after adding: {}", e);
            return Err(ServiceError::from(e));
        }

        self.reload_all().await?;
        Ok(())
    }
}

pub async fn run_subscription_sync(global: SharedGlobal, shutdown: tokio_util::sync::CancellationToken) {
    tracing::info!(
        "starting subscription sync task (interval={}s)",
        SUBSCRIPTION_SYNC_INTERVAL_SECS
    );

    let mut tick = time::interval(Duration::from_secs(SUBSCRIPTION_SYNC_INTERVAL_SECS));

    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = tick.tick() => {
                tracing::info!("running scheduled subscription sync");
                global.domain_rules.sync_subscriptions().await;
            }
            _ = shutdown.cancelled() => {
                tracing::info!("shutting down subscription sync task");
                break;
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum SubscriptionSyncError {
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("server returned {0}")]
    HttpStatus(reqwest::StatusCode),

    #[error("response exceeded size limit")]
    TooLarge,

    #[error("database error: {0}")]
    Database(#[from] DatabaseError),

    #[error("invalid format")]
    InvalidFormat,
}

impl From<SubscriptionSyncError> for ServiceError {
    fn from(e: SubscriptionSyncError) -> Self {
        match e {
            SubscriptionSyncError::TooLarge => {
                ServiceError::BadRequest("Subscription response exceeded size limit".into())
            }
            SubscriptionSyncError::HttpStatus(s) if s.is_client_error() => {
                ServiceError::BadRequest(format!("Subscription URL returned {s}"))
            }
            SubscriptionSyncError::Request(ref re) if re.is_connect() || re.is_timeout() => {
                ServiceError::BadRequest("Subscription URL is not reachable".into())
            }
            SubscriptionSyncError::InvalidFormat => {
                ServiceError::BadRequest("The content contains an unsupported format".into())
            }
            _ => ServiceError::Internal(e.into()),
        }
    }
}

/// Fetches the domain rules from a list subscription URL, and updates the database if there are changes.
pub async fn fetch_domain_rules_from_list_subscription_task(
    subscription: &ListSubscription,
    http_client: &reqwest::Client,
    db: &Arc<CoreDatabasePool>,
) -> Result<bool, SubscriptionSyncError> {
    let mut request = http_client.get(&subscription.url);

    // prefer ETag; fall back to Last-Modified if that's all we have
    if let Some(etag) = &subscription.etag {
        request = request.header(reqwest::header::IF_NONE_MATCH, etag);
    } else if let Some(lm) = &subscription.last_modified {
        request = request.header(reqwest::header::IF_MODIFIED_SINCE, lm);
    }

    let response = request.send().await?;

    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        tracing::debug!("list subscription {} unchanged (304), skipping sync", subscription.url);
        return Ok(false);
    }

    let status = response.status();
    if !status.is_success() {
        return Err(SubscriptionSyncError::HttpStatus(status));
    }

    if response
        .content_length()
        .is_some_and(|len| len > SUBSCRIPTION_MAX_RESPONSE_BYTES)
    {
        return Err(SubscriptionSyncError::TooLarge);
    }

    // extract cache headers before consuming the response body
    let etag = response
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let last_modified = response
        .headers()
        .get(reqwest::header::LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);

    let mut stream = response.bytes_stream();
    let mut total_bytes: u64 = 0;
    let mut domains: Vec<String> = Vec::new();
    let mut parser = reso_list::parser::ListParser::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        total_bytes += chunk.len() as u64;
        if total_bytes > SUBSCRIPTION_MAX_RESPONSE_BYTES {
            return Err(SubscriptionSyncError::TooLarge);
        }

        let text = String::from_utf8_lossy(&chunk);
        parser.push(&text, |domain| {
            if let Ok(normalized) = normalize_domain_pattern(domain) {
                domains.push(normalized);
            }
        });
    }

    let has_no_format = parser.format.is_none();

    parser.flush(|domain| {
        if let Ok(normalized) = normalize_domain_pattern(domain) {
            domains.push(normalized);
        }
    });

    // no format detected.
    if has_no_format && domains.is_empty() {
        return Err(SubscriptionSyncError::InvalidFormat);
    }

    if domains.is_empty() {
        tracing::warn!("list subscription {} contained no valid domains", subscription.url);
    }

    let count = DomainRule::sync_subscription(subscription.id.clone(), subscription.list_type, domains, db).await?;

    ListSubscription::update_after_sync(subscription.id.clone(), etag, last_modified, db).await?;

    tracing::info!(
        "synced {} domains from list subscription: '{}'",
        count,
        subscription.name
    );

    Ok(true)
}

/// Validates a list subscription URL.
fn validate_list_subscription_url(url: &str) -> Result<(), ServiceError> {
    let parsed = reqwest::Url::parse(url).map_err(|_| ServiceError::BadRequest("Invalid URL".into()))?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(ServiceError::BadRequest("URL scheme must be http or https".into()));
    }

    if parsed
        .host_str()
        .is_some_and(|str| str.ends_with(".local") || str == "localhost")
    {
        return Err(ServiceError::BadRequest("URL host cannot be a local domain".into()));
    }

    if let Ok(ip) = parsed.host_str().unwrap_or("").parse::<std::net::IpAddr>() {
        if ip.is_loopback() || ip.is_multicast() || ip.is_unspecified() {
            return Err(ServiceError::BadRequest(
                "URL host cannot be a loopback, multicast, or unspecified IP address".into(),
            ));
        }
        if let std::net::IpAddr::V4(ipv4) = ip
            && ipv4.is_private()
        {
            return Err(ServiceError::BadRequest(
                "URL host cannot be a private IP address".into(),
            ));
        } else if let std::net::IpAddr::V6(ipv6) = ip
            && ipv6.is_unique_local()
        {
            return Err(ServiceError::BadRequest(
                "URL host cannot be a unique local IPv6 address".into(),
            ));
        }
    }

    Ok(())
}
