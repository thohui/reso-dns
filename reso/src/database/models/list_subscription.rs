use crate::utils::now_millis;
use rusqlite::params;
use uuid::Uuid;

use serde::Serialize;

use crate::{
    database::models::ListAction,
    database::{CoreDatabasePool, DatabaseError},
    utils::uuid::EntityId,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ListSubscription {
    pub id: EntityId<Self>,
    pub name: String,
    pub url: String,
    pub list_type: ListAction,
    pub created_at: i64,
    pub enabled: bool,
    pub last_synced_at: Option<i64>,
    pub sync_enabled: bool,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

impl ListSubscription {
    pub fn new(name: String, url: String) -> Self {
        Self {
            id: EntityId::new(),
            name,
            url,
            list_type: ListAction::Block,
            created_at: now_millis(),
            enabled: true,
            last_synced_at: None,
            sync_enabled: true,
            etag: None,
            last_modified: None,
        }
    }
}

impl ListSubscription {
    pub async fn insert(self, db: &CoreDatabasePool) -> Result<(), DatabaseError> {
        db.interact(move |c| {
            c.execute(
                "INSERT INTO list_subscriptions (id, name, url, list_type, created_at, enabled, sync_enabled) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    self.id.id(),
                    self.name.as_str(),
                    self.url.as_str(),
                    self.list_type,
                    self.created_at,
                    self.enabled,
                    self.sync_enabled,
                ],
            )?;
            Ok(())
        })
        .await?;
        Ok(())
    }

    pub async fn list(db: &CoreDatabasePool) -> Result<Vec<Self>, DatabaseError> {
        db
            .interact(move |c| {
                let mut stmt = c.prepare(
                    "SELECT id, name, url, list_type, created_at, enabled, last_synced_at, sync_enabled, etag, last_modified FROM list_subscriptions",
                )?;
                let iter = stmt.query_map([], |r| {
                    Ok(Self {
                        id: EntityId::from(r.get::<_, Uuid>(0)?),
                        name: r.get(1)?,
                        url: r.get(2)?,
                        list_type: r.get(3)?,
                        created_at: r.get(4)?,
                        enabled: r.get(5)?,
                        last_synced_at: r.get(6)?,
                        sync_enabled: r.get(7)?,
                        etag: r.get(8)?,
                        last_modified: r.get(9)?,
                    })
                })?;
                iter.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await
    }

    pub async fn list_with_domain_counts(db: &CoreDatabasePool) -> Result<Vec<(Self, i64)>, DatabaseError> {
        db.interact(move |c| {
            let mut stmt = c.prepare(
                "SELECT ls.id, ls.name, ls.url, ls.list_type, ls.created_at, ls.enabled, ls.last_synced_at, ls.sync_enabled, ls.etag, ls.last_modified, COUNT(dr.id) \
                 FROM list_subscriptions ls \
                 LEFT JOIN domain_rules dr ON dr.subscription_id = ls.id \
                 GROUP BY ls.id",
            )?;
            let iter = stmt.query_map([], |r| {
                Ok((
                    Self {
                        id: EntityId::from(r.get::<_, Uuid>(0)?),
                        name: r.get(1)?,
                        url: r.get(2)?,
                        list_type: r.get(3)?,
                        created_at: r.get(4)?,
                        enabled: r.get(5)?,
                        last_synced_at: r.get(6)?,
                        sync_enabled: r.get(7)?,
                        etag: r.get(8)?,
                        last_modified: r.get(9)?,
                    },
                    r.get::<_, i64>(10)?,
                ))
            })?;
            iter.collect::<rusqlite::Result<Vec<_>>>()
        })
        .await
    }

    pub async fn delete_by_id(id: EntityId<Self>, db: &CoreDatabasePool) -> Result<bool, DatabaseError> {
        let rows = db
            .interact(move |c| c.execute("DELETE FROM list_subscriptions WHERE id = ?1", params![id.id()]))
            .await?;
        Ok(rows > 0)
    }

    pub async fn toggle_enabled(id: EntityId<ListSubscription>, db: &CoreDatabasePool) -> Result<bool, DatabaseError> {
        let rows = db
            .interact(move |c| {
                let tx = c.transaction()?;
                let rows = tx.execute(
                    "UPDATE list_subscriptions SET enabled = NOT enabled WHERE id = ?1",
                    params![id.id()],
                )?;
                tx.execute(
                    "UPDATE domain_rules SET enabled = (SELECT enabled FROM list_subscriptions WHERE id = ?1) WHERE subscription_id = ?1",
                    params![id.id()],
                )?;
                tx.commit()?;
                Ok(rows)
            })
            .await?;
        Ok(rows > 0)
    }

    /// Toggles the sync_enabled state of a subscription.
    pub async fn toggle_sync_enabled(
        id: EntityId<ListSubscription>,
        db: &CoreDatabasePool,
    ) -> Result<bool, DatabaseError> {
        let rows = db
            .interact(move |c| {
                c.execute(
                    "UPDATE list_subscriptions SET sync_enabled = NOT sync_enabled WHERE id = ?1",
                    params![id.id()],
                )?;
                Ok(c.changes())
            })
            .await?;
        Ok(rows > 0)
    }

    pub async fn update_after_sync(
        id: EntityId<ListSubscription>,
        etag: Option<String>,
        last_modified: Option<String>,
        db: &CoreDatabasePool,
    ) -> Result<(), DatabaseError> {
        let now = now_millis();
        db.interact(move |c| {
            c.execute(
                "UPDATE list_subscriptions SET last_synced_at = ?1, etag = ?2, last_modified = ?3 WHERE id = ?4",
                params![now, etag, last_modified, id.id()],
            )?;
            Ok(())
        })
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{models::domain_rule::DomainRule, setup_core_test_db};

    #[tokio::test]
    async fn test_insert_and_list() {
        let db = setup_core_test_db().await.unwrap();
        let sub = ListSubscription::new("Test List".into(), "https://example.com/list.txt".into());
        sub.clone().insert(&db.conn).await.unwrap();

        let subs = ListSubscription::list(&db.conn).await.unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0], sub);
    }

    #[tokio::test]
    async fn test_delete() {
        let db = setup_core_test_db().await.unwrap();
        let sub = ListSubscription::new("To Delete".into(), "https://example.com/del.txt".into());
        sub.clone().insert(&db.conn).await.unwrap();

        assert_eq!(ListSubscription::list(&db.conn).await.unwrap().len(), 1);
        ListSubscription::delete_by_id(sub.id, &db.conn).await.unwrap();
        assert_eq!(ListSubscription::list(&db.conn).await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_toggle_enabled_cascades_to_domain_rules() {
        use crate::database::models::domain_rule::DomainRule;

        let db = setup_core_test_db().await.unwrap();
        let sub = ListSubscription::new("Toggle".into(), "https://example.com/toggle.txt".into());
        sub.clone().insert(&db.conn).await.unwrap();

        DomainRule::sync_subscription(
            sub.id.clone(),
            vec![
                (
                    "a.com".into(),
                    crate::database::models::MatchType::Domain,
                    ListAction::Block,
                ),
                (
                    "b.com".into(),
                    crate::database::models::MatchType::Domain,
                    ListAction::Block,
                ),
            ],
            &db.conn,
        )
        .await
        .unwrap();

        let rules = DomainRule::list_all(&db.conn).await.unwrap();
        assert!(rules.iter().all(|d| d.enabled));

        ListSubscription::toggle_enabled(sub.id.clone(), &db.conn)
            .await
            .unwrap();
        let rules = DomainRule::list_all(&db.conn).await.unwrap();
        assert!(rules.iter().all(|d| !d.enabled));

        ListSubscription::toggle_enabled(sub.id, &db.conn).await.unwrap();
        let rules = DomainRule::list_all(&db.conn).await.unwrap();
        assert!(rules.iter().all(|d| d.enabled));
    }

    #[tokio::test]
    async fn test_update_after_sync() {
        let db = setup_core_test_db().await.unwrap();
        let sub = ListSubscription::new("Sync Test".into(), "https://example.com/sync.txt".into());
        sub.clone().insert(&db.conn).await.unwrap();

        let before = ListSubscription::list(&db.conn).await.unwrap();
        assert!(before[0].last_synced_at.is_none());

        ListSubscription::update_after_sync(sub.id, None, None, &db.conn)
            .await
            .unwrap();

        let after = ListSubscription::list(&db.conn).await.unwrap();
        assert!(after[0].last_synced_at.is_some());
    }

    #[tokio::test]
    async fn test_duplicate_url_fails() {
        let db = setup_core_test_db().await.unwrap();
        ListSubscription::new("A".into(), "https://example.com/list.txt".into())
            .insert(&db.conn)
            .await
            .unwrap();

        let result = ListSubscription::new("B".into(), "https://example.com/list.txt".into())
            .insert(&db.conn)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_with_domain_counts() {
        let db = setup_core_test_db().await.unwrap();
        let sub = ListSubscription::new("Test List".into(), "https://example.com/list.txt".into());
        sub.clone().insert(&db.conn).await.unwrap();

        let mut dn = DomainRule::new("test.com".into());
        dn.subscription_id = Some(sub.id.clone());
        dn.insert(&db.conn).await.unwrap();

        let subs = ListSubscription::list_with_domain_counts(&db.conn).await.unwrap();
        let (fetched_sub, domain_count) = subs.first().unwrap();
        assert!(fetched_sub == &sub);
        assert!(*domain_count == 1)
    }
}
