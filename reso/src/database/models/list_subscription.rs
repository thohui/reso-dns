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
    pub domain_count: i64,
    pub hash: Option<String>,
    pub sync_enabled: bool,
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
            domain_count: 0,
            hash: None,
            sync_enabled: true,
        }
    }
}

impl ListSubscription {
    pub async fn insert(self, db: &CoreDatabasePool) -> Result<(), DatabaseError> {
        db.interact(move |c| {
            c.execute(
                "INSERT INTO list_subscriptions (id, name, url, list_type, created_at, enabled, hash, sync_enabled) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    self.id.id(),
                    self.name.as_str(),
                    self.url.as_str(),
                    self.list_type,
                    self.created_at,
                    self.enabled,
                    self.hash.as_deref(),
                    self.sync_enabled,
                ],
            )?;
            Ok(())
        })
        .await?;
        Ok(())
    }

    pub async fn list(db: &CoreDatabasePool) -> Result<Vec<Self>, DatabaseError> {
        Ok(db
            .interact(move |c| {
                let mut stmt = c.prepare(
                    "SELECT id, name, url, list_type, created_at, enabled, last_synced_at, domain_count, hash, sync_enabled FROM list_subscriptions",
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
                        domain_count: r.get(7)?,
                        hash: r.get(8)?,
                        sync_enabled: r.get(9)?,
                    })
                })?;
                iter.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await?)
    }

    pub async fn delete(self, db: &CoreDatabasePool) -> Result<bool, DatabaseError> {
        Self::delete_by_id(self.id, db).await
    }

    pub async fn delete_by_id(id: EntityId<Self>, db: &CoreDatabasePool) -> Result<bool, DatabaseError> {
        let rows = db
            .interact(move |c| Ok(c.execute("DELETE FROM list_subscriptions WHERE id = ?1", params![id.id()])?))
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
        domain_count: i64,
        hash: String,
        db: &CoreDatabasePool,
    ) -> Result<(), DatabaseError> {
        let now = now_millis();
        db.interact(move |c| {
            c.execute(
                "UPDATE list_subscriptions SET last_synced_at = ?1, domain_count = ?2, hash = ?3 WHERE id = ?4",
                params![now, domain_count, hash, id.id()],
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
    use crate::database::setup_core_test_db;

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
        sub.delete(&db.conn).await.unwrap();
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
            ListAction::Block,
            vec!["a.com".into(), "b.com".into()],
            &db.conn,
        )
        .await
        .unwrap();

        // All domain rules enabled by default
        let rules = DomainRule::list_all(&db.conn).await.unwrap();
        assert!(rules.iter().all(|d| d.enabled));

        // Disable subscription — rules should follow
        ListSubscription::toggle_enabled(sub.id.clone(), &db.conn)
            .await
            .unwrap();
        let rules = DomainRule::list_all(&db.conn).await.unwrap();
        assert!(rules.iter().all(|d| !d.enabled));

        // Re-enable — rules should follow
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
        assert_eq!(before[0].domain_count, 0);

        ListSubscription::update_after_sync(sub.id, 42, "abc123".into(), &db.conn)
            .await
            .unwrap();

        let after = ListSubscription::list(&db.conn).await.unwrap();
        assert!(after[0].last_synced_at.is_some());
        assert_eq!(after[0].domain_count, 42);
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
}
