use rusqlite::params;
use uuid::Uuid;

use serde::Serialize;

use crate::{
    database::{CoreDatabasePool, DatabaseError},
    time::now_millis,
    uuid::EntityId,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ListSubscription {
    pub id: EntityId<Self>,
    pub name: String,
    pub url: String,
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
            created_at: now_millis(),
            enabled: true,
            last_synced_at: None,
            sync_enabled: true,
            etag: None,
            last_modified: None,
        }
    }
}

pub async fn insert(db: &CoreDatabasePool, list_subscription: ListSubscription) -> Result<(), DatabaseError> {
    db.interact(move |c| {
        c.execute(
            "INSERT INTO list_subscriptions(id, name, url, created_at, enabled, sync_enabled) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                list_subscription.id.id(),
                list_subscription.name.as_str(),
                list_subscription.url.as_str(),
                list_subscription.created_at,
                list_subscription.enabled,
                list_subscription.sync_enabled,
            ],
        )?;
        Ok(())
    })
    .await?;
    Ok(())
}

pub async fn list(db: &CoreDatabasePool) -> Result<Vec<ListSubscription>, DatabaseError> {
    db.interact(move |c| {
        let mut stmt = c.prepare(
            "SELECT id, name, url, created_at, enabled, last_synced_at, sync_enabled, etag, last_modified FROM list_subscriptions",
        )?;
        let iter = stmt.query_map([], |r| {
            Ok(ListSubscription {
                id: EntityId::from(r.get::<_, Uuid>(0)?),
                name: r.get(1)?,
                url: r.get(2)?,
                created_at: r.get(3)?,
                enabled: r.get(4)?,
                last_synced_at: r.get(5)?,
                sync_enabled: r.get(6)?,
                etag: r.get(7)?,
                last_modified: r.get(8)?,
            })
        })?;
        iter.collect::<rusqlite::Result<Vec<_>>>()
    })
    .await
}

pub async fn list_with_domain_counts(db: &CoreDatabasePool) -> Result<Vec<(ListSubscription, i64)>, DatabaseError> {
    db.interact(move |c| {
            let mut stmt = c.prepare(
                "SELECT ls.id, ls.name, ls.url, ls.created_at, ls.enabled, ls.last_synced_at, ls.sync_enabled, ls.etag, ls.last_modified, COUNT(dr.id)
                 FROM list_subscriptions ls
                 LEFT JOIN domain_rules dr ON dr.subscription_id = ls.id
                 GROUP BY ls.id",
            )?;
            let iter = stmt.query_map([], |r| {
                Ok((
                    ListSubscription {
                        id: EntityId::from(r.get::<_, Uuid>(0)?),
                        name: r.get(1)?,
                        url: r.get(2)?,
                        created_at: r.get(3)?,
                        enabled: r.get(4)?,
                        last_synced_at: r.get(5)?,
                        sync_enabled: r.get(6)?,
                        etag: r.get(7)?,
                        last_modified: r.get(8)?,
                    },
                    r.get::<_, i64>(9)?,
                ))
            })?;
            iter.collect::<rusqlite::Result<Vec<_>>>()
        })
        .await
}

pub async fn delete_by_id(db: &CoreDatabasePool, id: EntityId<ListSubscription>) -> Result<bool, DatabaseError> {
    let rows = db
        .interact(move |c| c.execute("DELETE FROM list_subscriptions WHERE id = ?1", params![id.id()]))
        .await?;
    Ok(rows > 0)
}

pub async fn toggle_enabled(db: &CoreDatabasePool, id: EntityId<ListSubscription>) -> Result<bool, DatabaseError> {
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

pub async fn toggle_sync_enabled(db: &CoreDatabasePool, id: EntityId<ListSubscription>) -> Result<bool, DatabaseError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{
        models::{
            ListAction,
            domain_rule::{self, DomainRule},
        },
        setup_core_test_db,
    };

    #[tokio::test]
    async fn test_insert_and_list() {
        let db = setup_core_test_db().await.unwrap();
        let sub = ListSubscription::new("Test List".into(), "https://example.com/list.txt".into());
        insert(&db.conn, sub.clone()).await.unwrap();

        let subs = list(&db.conn).await.unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0], sub);
    }

    #[tokio::test]
    async fn test_delete() {
        let db = setup_core_test_db().await.unwrap();
        let sub = ListSubscription::new("To Delete".into(), "https://example.com/del.txt".into());
        insert(&db.conn, sub.clone()).await.unwrap();

        assert_eq!(list(&db.conn).await.unwrap().len(), 1);
        delete_by_id(&db.conn, sub.id).await.unwrap();
        assert_eq!(list(&db.conn).await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_toggle_enabled_cascades_to_domain_rules() {
        use crate::database::models::domain_rule::sync_subscription;

        let db = setup_core_test_db().await.unwrap();
        let sub = ListSubscription::new("Toggle".into(), "https://example.com/toggle.txt".into());
        insert(&db.conn, sub.clone()).await.unwrap();

        sync_subscription(
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

        let rules = domain_rule::list_all(&db.conn).await.unwrap();
        assert!(rules.iter().all(|d| d.enabled));

        toggle_enabled(&db.conn, sub.id.clone()).await.unwrap();
        let rules = domain_rule::list_all(&db.conn).await.unwrap();
        assert!(rules.iter().all(|d| !d.enabled));

        toggle_enabled(&db.conn, sub.id).await.unwrap();
        let rules = domain_rule::list_all(&db.conn).await.unwrap();
        assert!(rules.iter().all(|d| d.enabled));
    }

    #[tokio::test]
    async fn test_update_after_sync() {
        let db = setup_core_test_db().await.unwrap();
        let sub = ListSubscription::new("Sync Test".into(), "https://example.com/sync.txt".into());
        insert(&db.conn, sub.clone()).await.unwrap();

        let before = list(&db.conn).await.unwrap();
        assert!(before[0].last_synced_at.is_none());

        update_after_sync(sub.id, None, None, &db.conn).await.unwrap();

        let after = list(&db.conn).await.unwrap();
        assert!(after[0].last_synced_at.is_some());
    }

    #[tokio::test]
    async fn test_list_with_domain_counts() {
        let db = setup_core_test_db().await.unwrap();
        let sub = ListSubscription::new("Test List".into(), "https://example.com/list.txt".into());
        insert(&db.conn, sub.clone()).await.unwrap();

        let mut dn = DomainRule::new("test.com".into());
        dn.subscription_id = Some(sub.id.clone());
        domain_rule::insert(&db.conn, dn).await.unwrap();

        let subs = list_with_domain_counts(&db.conn).await.unwrap();
        let (fetched_sub, domain_count) = subs.first().unwrap();
        assert!(*fetched_sub == sub);
        assert!(*domain_count == 1)
    }

    #[tokio::test]
    async fn test_duplicate_url_fails() {
        let db = setup_core_test_db().await.unwrap();
        insert(
            &db.conn,
            ListSubscription::new("A".into(), "https://example.com/list.txt".into()),
        )
        .await
        .unwrap();

        let result = insert(
            &db.conn,
            ListSubscription::new("B".into(), "https://example.com/list.txt".into()),
        )
        .await;
        assert!(result.is_err());
    }
}
