use futures::StreamExt;
use std::{sync::Arc, time::Duration};

const SUBSCRIPTION_FETCH_TIMEOUT_SECS: u64 = 30;
const SUBSCRIPTION_MAX_RESPONSE_BYTES: u64 = 5 * 1024 * 1024; // 5 MB

use arc_swap::ArcSwap;
use reso_dns::domain_name::DomainName;
use reso_list::DomainListMatcher;
use tokio::time::{self, MissedTickBehavior};

use crate::{
    database::{
        CoreDatabasePool,
        models::{ListAction, domain_rule::DomainRule, list_subscription::ListSubscription},
    },
    global::SharedGlobal,
    utils::uuid::EntityId,
};

use super::ServiceError;

const SUBSCRIPTION_SYNC_INTERVAL_SECS: u64 = 60 * 60 * 24; // 24 hours

/// Validates and normalizes a domain pattern, supporting `*.example.com` wildcards.
/// Returns the canonical stored form, e.g. `*.example.com` or `example.com`.
fn normalize_domain_pattern(input: &str) -> Result<String, ServiceError> {
    // what we are doing here is kinda hacky, because DomainName doesn't support wildcard patterns, but we want to reuse its validation and normalization logic.
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
    pub async fn load(db: &CoreDatabasePool) -> anyhow::Result<Self> {
        let domains = DomainRule::list_all(db).await?;
        let (allow_list, block_list): (Vec<_>, Vec<_>) = domains.iter().partition(|d| d.action == ListAction::Allow);
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

pub struct DomainRulesService {
    matchers: ArcSwap<Matchers>,
    connection: Arc<CoreDatabasePool>,
    http_client: reqwest::Client,
}

impl DomainRulesService {
    pub async fn initialize(connection: Arc<CoreDatabasePool>) -> anyhow::Result<Self> {
        Ok(Self {
            matchers: ArcSwap::new(Matchers::load(&connection).await?.into()),
            connection,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(SUBSCRIPTION_FETCH_TIMEOUT_SECS))
                .build()?,
        })
    }

    /// Adds a new domain rule with the given domain pattern and action.
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

    /// Removes a domain rule by domain pattern.
    pub async fn remove_domain(&self, domain: &str) -> Result<(), ServiceError> {
        let domain = normalize_domain_pattern(domain)?;

        let changed = DomainRule::delete_by_domain(&domain, &self.connection).await?;

        if !changed {
            return Err(ServiceError::NotFound("Domain not found".into()));
        }

        self.reload_all().await?;
        Ok(())
    }

    /// Updates the action of an existing domain rule.
    pub async fn update_domain_action(&self, domain: &str, action: ListAction) -> Result<(), ServiceError> {
        let domain = normalize_domain_pattern(domain)?;

        let changed = DomainRule::update_action(&domain, action, &self.connection).await?;

        if !changed {
            return Err(ServiceError::NotFound("Domain not found".into()));
        }

        self.reload_all().await?;
        Ok(())
    }

    /// Toggles the enabled state of an individual domain rule.
    pub async fn toggle_domain(&self, domain: &str) -> Result<(), ServiceError> {
        let domain = normalize_domain_pattern(domain)?;

        let changed = DomainRule::toggle(&domain, &self.connection).await?;

        if !changed {
            return Err(ServiceError::NotFound("Domain not found".into()));
        }

        self.reload_all().await?;

        Ok(())
    }

    async fn reload_all(&self) -> Result<(), ServiceError> {
        self.matchers.swap(
            Matchers::load(&self.connection)
                .await
                .map_err(ServiceError::Internal)?
                .into(),
        );
        Ok(())
    }

    async fn reload_allow_list(&self) -> Result<(), ServiceError> {
        let rules = DomainRule::list_enabled_by_action(ListAction::Allow, &self.connection).await?;

        let new_matcher = Arc::new(
            DomainListMatcher::load(rules.iter().map(|r| r.domain.as_str()))
                .map_err(|e| ServiceError::Internal(e.into()))?,
        );

        self.matchers.rcu(|current| {
            Arc::new(Matchers {
                allow_list_matcher: Arc::clone(&new_matcher),
                blocklist_matcher: Arc::clone(&current.blocklist_matcher),
            })
        });

        Ok(())
    }

    async fn reload_blocklist(&self) -> Result<(), ServiceError> {
        let rules = DomainRule::list_enabled_by_action(ListAction::Block, &self.connection).await?;
        let new_matcher = Arc::new(
            DomainListMatcher::load(rules.iter().map(|r| r.domain.as_str()))
                .map_err(|e| ServiceError::Internal(e.into()))?,
        );

        self.matchers.rcu(|current| {
            Arc::new(Matchers {
                blocklist_matcher: Arc::clone(&new_matcher),
                allow_list_matcher: Arc::clone(&current.allow_list_matcher),
            })
        });

        Ok(())
    }

    /// Checks if a given domain is blocked by the current rules.
    pub fn is_blocked(&self, name: &str) -> bool {
        let matchers = self.matchers.load();
        if matchers.blocklist_matcher.exists(name) {
            return !matchers.allow_list_matcher.exists(name);
        }
        false
    }

    /// Lists all subscriptions.
    pub async fn list_subscriptions(&self) -> Result<Vec<ListSubscription>, ServiceError> {
        Ok(ListSubscription::list(&self.connection).await?)
    }

    /// Removes a list subscription by ID.
    pub async fn remove_list_subscription(&self, id: EntityId<ListSubscription>) -> Result<(), ServiceError> {
        let changed = ListSubscription::delete_by_id(id, &self.connection).await?;
        if !changed {
            return Err(ServiceError::NotFound("Subscription not found".into()));
        }
        self.reload_all().await?;
        Ok(())
    }

    /// Toggles the enabled state of a list subscription by ID.
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

    /// Syncs all list subscriptions, fetching updated rules from their URLs if needed.
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

    /// Adds a new list subscription with the given URL and list type.
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

        if let Some(content_type) = content_type {
            if !content_type
                .to_str()
                .map_err(|_| ServiceError::BadRequest("Invalid content-type header from URL".into()))?
                .contains("text/plain")
            {
                return Err(ServiceError::BadRequest(
                    "URL must have content-type of text/plain".into(),
                ));
            }
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
            tracing::error!("failed to fetch domain rules from subscription after adding: {}", e);
            return Err(ServiceError::Internal(anyhow::anyhow!(
                "Failed to fetch domain rules from subscription after adding",
            )));
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

/// Fetches the domain rules from a list subscription URL, and updates the database if there are changes.
pub async fn fetch_domain_rules_from_list_subscription_task(
    subscription: &ListSubscription,
    http_client: &reqwest::Client,
    db: &Arc<CoreDatabasePool>,
) -> anyhow::Result<bool> {
    let response = match http_client.get(&subscription.url).send().await {
        Ok(r) => r,
        Err(e) => {
            anyhow::bail!("failed to fetch list subscription: {}: {:?}", subscription.url, e);
        }
    };

    if let Err(e) = response.error_for_status_ref() {
        anyhow::bail!("list subscription returned error status: {}: {:?}", subscription.url, e);
    }

    let content_length = response.content_length();

    let mut stream = response.bytes_stream();
    let initial_capacity: usize = content_length.map(|len| len as usize).unwrap_or(64 * 1024); // 64KB
    let mut buf = Vec::with_capacity(initial_capacity);

    // read the response stream in chunks, so we can enforce the max size limit better
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buf.extend_from_slice(&chunk);
        if buf.len() as u64 > SUBSCRIPTION_MAX_RESPONSE_BYTES {
            anyhow::bail!(
                "list subscription {} response exceeded size limit during streaming ({} bytes), aborting",
                subscription.url,
                buf.len()
            );
        }
    }

    let text = match String::from_utf8(buf) {
        Ok(t) => t,
        Err(e) => anyhow::bail!("list subscription {} is not valid UTF-8: {:?}", subscription.url, e),
    };

    let content_hash: String = reso_list::parser::calculate_hash(&text);

    if subscription.hash.as_deref() == Some(&content_hash) {
        tracing::debug!("list subscription {} unchanged, skipping sync", subscription.url);
        return Ok(false);
    }

    let domains: Vec<String> = reso_list::parser::ListParser::new(&text)
        .parse()
        .into_iter()
        .map(str::to_owned)
        .filter_map(|d| normalize_domain_pattern(&d).ok())
        .collect();

    if domains.is_empty() {
        tracing::warn!("list subscription {} contained no valid domains", subscription.url);
    }

    let count =
        match DomainRule::sync_subscription(subscription.id.clone(), subscription.list_type.clone(), domains, db).await
        {
            Ok(count) => count,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "failed to sync domain rules from subscription {}: {:?}",
                    subscription.url,
                    e
                ));
            }
        };

    if let Err(e) = ListSubscription::update_after_sync(subscription.id.clone(), count, content_hash, db).await {
        anyhow::bail!(
            "failed to update list subscription {} after sync: {:?}",
            subscription.url,
            e
        );
    }

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
        if let std::net::IpAddr::V4(ipv4) = ip {
            if ipv4.is_private() {
                return Err(ServiceError::BadRequest(
                    "URL host cannot be a private IP address".into(),
                ));
            }
        } else if let std::net::IpAddr::V6(ipv6) = ip {
            if ipv6.is_unique_local() {
                return Err(ServiceError::BadRequest(
                    "URL host cannot be a unique local IPv6 address".into(),
                ));
            }
        }
    }

    Ok(())
}
