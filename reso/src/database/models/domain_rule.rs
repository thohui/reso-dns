use rusqlite::params;
use serde::Serialize;
use uuid::Uuid;

use rusqlite::types::Value;

use crate::{
    database::{
        CoreDatabasePool, DatabaseError,
        models::{ListAction, list_subscription::ListSubscription},
        query::WhereBuilder,
    },
    utils::{now_millis, uuid::EntityId},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct DomainRule {
    pub id: EntityId<Self>,
    pub domain: String,
    pub action: ListAction,
    pub created_at: i64,
    pub enabled: bool,
    pub subscription_id: Option<EntityId<ListSubscription>>,
}

impl DomainRule {
    pub fn new(domain: String) -> Self {
        Self {
            id: EntityId::new(),
            domain,
            action: ListAction::Block,
            created_at: now_millis(),
            enabled: true,
            subscription_id: None,
        }
    }
}

impl DomainRule {
    /// Inserts this domain rule into the database.
    pub async fn insert(self, db: &CoreDatabasePool) -> Result<(), DatabaseError> {
        db.interact(move |c| {
            c.execute(
                "INSERT INTO domain_rules (id, domain, action, created_at, enabled, subscription_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    self.id.id(),
                    self.domain.as_str(),
                    self.action,
                    self.created_at,
                    self.enabled,
                    self.subscription_id.as_ref().map(|id| *id.id()),
                ],
            )?;
            Ok(())
        })
        .await?;
        Ok(())
    }

    /// Deletes this domain rule from the database.
    pub async fn delete(self, db: &CoreDatabasePool) -> Result<bool, DatabaseError> {
        let rows = db
            .interact(move |c| Ok(c.execute("DELETE FROM domain_rules WHERE domain = ?1", params![self.domain])?))
            .await?;
        Ok(rows > 0)
    }

    /// Deletes a domain rule by domain name.
    pub async fn delete_by_domain(domain: &str, db: &CoreDatabasePool) -> Result<bool, DatabaseError> {
        let domain = domain.to_string();
        let rows = db
            .interact(move |c| Ok(c.execute("DELETE FROM domain_rules WHERE domain = ?1", params![domain])?))
            .await?;
        Ok(rows > 0)
    }

    /// Lists domain rules with pagination and optional search by domain name.
    pub async fn list(
        db: &CoreDatabasePool,
        limit: i64,
        offset: i64,
        search: Option<String>,
    ) -> Result<Vec<Self>, DatabaseError> {
        Ok(db
            .interact(move |c| {
                let mut b = WhereBuilder::new(2);
                if let Some(ref s) = search {
                    b.like("domain", s);
                }
                let (where_clause, filter_params) = b.build();

                let sql = format!(
                    r#"
                    SELECT id, domain, action, created_at, enabled, subscription_id
                    FROM domain_rules
                    WHERE 1=1 {where_clause}
                    ORDER BY created_at DESC
                    LIMIT ?1 OFFSET ?2
                    "#
                );
                let mut list_params: Vec<Value> = vec![Value::Integer(limit), Value::Integer(offset)];
                list_params.extend(filter_params);

                let mut stmt = c.prepare(&sql)?;
                let iter = stmt.query_map(rusqlite::params_from_iter(&list_params), |r| {
                    Ok(DomainRule {
                        id: EntityId::from(r.get::<_, Uuid>(0)?),
                        domain: r.get(1)?,
                        action: r.get(2)?,
                        created_at: r.get(3)?,
                        enabled: r.get(4)?,
                        subscription_id: r.get::<_, Option<Uuid>>(5)?.map(EntityId::from),
                    })
                })?;
                iter.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await?)
    }

    /// Lists all enabled domain rules for a given action. Used for building matchers.
    pub async fn list_enabled_by_action(action: ListAction, db: &CoreDatabasePool) -> Result<Vec<Self>, DatabaseError> {
        db.interact(move |c| {
            let mut stmt = c.prepare(
                "SELECT id, domain, action, created_at, enabled, subscription_id \
                 FROM domain_rules WHERE action = ?1 AND enabled = 1 ORDER BY created_at",
            )?;
            let iter = stmt.query_map(params![action], |r| {
                Ok(DomainRule {
                    id: EntityId::from(r.get::<_, Uuid>(0)?),
                    domain: r.get(1)?,
                    action: r.get(2)?,
                    created_at: r.get(3)?,
                    enabled: r.get(4)?,
                    subscription_id: r.get::<_, Option<Uuid>>(5)?.map(EntityId::from),
                })
            })?;
            iter.collect::<rusqlite::Result<Vec<_>>>()
        })
        .await
    }

    /// Lists all domain rules without pagination.
    pub async fn list_all(db: &CoreDatabasePool) -> Result<Vec<Self>, DatabaseError> {
        Ok(db
            .interact(move |c| {
                let mut stmt = c.prepare(
                    r#"
                    SELECT id, domain, action, created_at, enabled, subscription_id
                    FROM domain_rules
                    ORDER BY created_at
                    "#,
                )?;
                let iter = stmt.query_map([], |r| {
                    Ok(DomainRule {
                        id: EntityId::from(r.get::<_, Uuid>(0)?),
                        domain: r.get(1)?,
                        action: r.get(2)?,
                        created_at: r.get(3)?,
                        enabled: r.get(4)?,
                        subscription_id: r.get::<_, Option<Uuid>>(5)?.map(EntityId::from),
                    })
                })?;
                iter.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await?)
    }

    /// Updates the action of a domain rule. It will also clear the subscription_id to disassociate it from any subscription.
    pub async fn update_action(domain: &str, action: ListAction, db: &CoreDatabasePool) -> Result<bool, DatabaseError> {
        let domain = domain.to_string();
        let rows = db
            .interact(move |c| {
                Ok(c.execute(
                    "UPDATE domain_rules SET action = ?1, subscription_id = NULL WHERE domain = ?2",
                    params![action, domain],
                )?)
            })
            .await?;
        Ok(rows > 0)
    }

    /// Toggles the enabled state of a domain rule. If the rule is enabled, it will be disabled, and vice versa.
    pub async fn toggle(domain: &str, db: &CoreDatabasePool) -> Result<bool, DatabaseError> {
        let domain = domain.to_string();
        let rows = db
            .interact(move |c| {
                Ok(c.execute(
                    "UPDATE domain_rules SET enabled = NOT enabled WHERE domain = ?1",
                    params![domain],
                )?)
            })
            .await?;
        Ok(rows > 0)
    }

    /// Syncs a list of domains for a subscription. It will insert new domains, delete removed domains, and update existing ones.
    pub async fn sync_subscription(
        subscription_id: EntityId<ListSubscription>,
        action: ListAction,
        domains: Vec<String>,
        db: &CoreDatabasePool,
    ) -> Result<i64, DatabaseError> {
        let now = now_millis();
        Ok(db
            .interact(move |c| {
                let tx = c.transaction()?;

                // using a temporary table to reduce the number of queries.
                // sqlite has a variable limit which would be hit if we directly insert into domain_rules with a large list.
                tx.execute_batch(
                    "CREATE TEMP TABLE IF NOT EXISTS temp.domain_rules_sync_staging (domain TEXT PRIMARY KEY)",
                )?;
                tx.execute_batch("DELETE FROM temp.domain_rules_sync_staging")?;

                {
                    let mut stmt = tx.prepare("INSERT OR IGNORE INTO temp.domain_rules_sync_staging VALUES (?1)")?;
                    for domain in &domains {
                        stmt.execute(params![domain])?;
                    }
                }

                tx.execute(
                    "DELETE FROM domain_rules WHERE subscription_id = ?1 AND domain NOT IN (SELECT domain FROM temp.domain_rules_sync_staging)",
                    params![subscription_id.id()],
                )?;

                {
                    let new_ids: Vec<(Uuid, String)> = {
                        let mut stmt = tx.prepare(
                            "SELECT s.domain FROM temp.domain_rules_sync_staging s WHERE NOT EXISTS (SELECT 1 FROM domain_rules WHERE domain = s.domain)",
                        )?;
                        stmt.query_map([], |r| r.get::<_, String>(0))?
                            .collect::<rusqlite::Result<Vec<_>>>()?
                            .into_iter()
                            .map(|d| (Uuid::now_v7(), d))
                            .collect()
                    };

                    let mut stmt = tx.prepare(
                        "INSERT INTO domain_rules (id, domain, action, created_at, enabled, subscription_id) VALUES (?1, ?2, ?3, ?4, 1, ?5)",
                    )?;
                    for (id, domain) in &new_ids {
                        stmt.execute(params![id, domain.as_str(), &action, now, subscription_id.id()])?;
                    }
                }

                let count: i64 = tx.query_row(
                    "SELECT COUNT(*) FROM domain_rules WHERE subscription_id = ?1",
                    params![subscription_id.id()],
                    |r| r.get(0),
                )?;

                tx.commit()?;
                Ok(count)
            })
            .await?)
    }

    /// Counts the total number of domain rules, optionally filtered by a search term.
    pub async fn row_count(db: &CoreDatabasePool, search: Option<String>) -> Result<i64, DatabaseError> {
        Ok(db
            .interact(move |c| {
                let mut b = WhereBuilder::new(0);
                if let Some(ref s) = search {
                    b.like("domain", s);
                }
                let (where_clause, filter_params) = b.build();
                let sql = format!("SELECT COUNT(*) FROM domain_rules WHERE 1=1 {where_clause}");
                c.query_row(&sql, rusqlite::params_from_iter(&filter_params), |r| r.get(0))
            })
            .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{models::list_subscription::ListSubscription, setup_core_test_db};

    #[tokio::test]
    async fn test_insert_and_list() {
        let db = setup_core_test_db().await.unwrap();
        let rule = DomainRule::new("google.com".into());
        rule.clone().insert(&db.conn).await.unwrap();

        let rules = DomainRule::list(&db.conn, 10, 0, None).await.unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0], rule);
    }

    #[tokio::test]
    async fn test_list_pagination() {
        let db = setup_core_test_db().await.unwrap();
        for i in 0..5 {
            DomainRule::new(format!("domain{i}.com"))
                .insert(&db.conn)
                .await
                .unwrap();
        }

        let page1 = DomainRule::list(&db.conn, 2, 0, None).await.unwrap();
        assert_eq!(page1.len(), 2);

        let page2 = DomainRule::list(&db.conn, 2, 2, None).await.unwrap();
        assert_eq!(page2.len(), 2);

        let page3 = DomainRule::list(&db.conn, 2, 4, None).await.unwrap();
        assert_eq!(page3.len(), 1);
    }

    #[tokio::test]
    async fn test_delete() {
        let db = setup_core_test_db().await.unwrap();
        let rule = DomainRule::new("delete-me.com".into());
        rule.clone().insert(&db.conn).await.unwrap();

        assert_eq!(DomainRule::row_count(&db.conn, None).await.unwrap(), 1);
        rule.delete(&db.conn).await.unwrap();
        assert_eq!(DomainRule::row_count(&db.conn, None).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_toggle() {
        let db = setup_core_test_db().await.unwrap();
        DomainRule::new("toggle.com".into()).insert(&db.conn).await.unwrap();

        let before = DomainRule::list(&db.conn, 1, 0, None).await.unwrap();
        assert!(before[0].enabled);

        DomainRule::toggle("toggle.com", &db.conn).await.unwrap();

        let after = DomainRule::list(&db.conn, 1, 0, None).await.unwrap();
        assert!(!after[0].enabled);

        DomainRule::toggle("toggle.com", &db.conn).await.unwrap();

        let restored = DomainRule::list(&db.conn, 1, 0, None).await.unwrap();
        assert!(restored[0].enabled);
    }

    #[tokio::test]
    async fn test_duplicate_insert_fails() {
        let db = setup_core_test_db().await.unwrap();
        DomainRule::new("dup.com".into()).insert(&db.conn).await.unwrap();

        let result = DomainRule::new("dup.com".into()).insert(&db.conn).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sync_subscription() {
        let db = setup_core_test_db().await.unwrap();
        let sub = ListSubscription::new("Test".into(), "https://example.com/list.txt".into());
        sub.clone().insert(&db.conn).await.unwrap();

        // first sync
        let count = DomainRule::sync_subscription(
            sub.id.clone(),
            ListAction::Block,
            vec!["a.com".into(), "b.com".into(), "c.com".into()],
            &db.conn,
        )
        .await
        .unwrap();
        assert_eq!(count, 3);

        // second sync: one new, one removed, one unchanged
        let count = DomainRule::sync_subscription(
            sub.id,
            ListAction::Block,
            vec!["a.com".into(), "b.com".into(), "d.com".into()],
            &db.conn,
        )
        .await
        .unwrap();
        assert_eq!(count, 3);

        let all = DomainRule::list_all(&db.conn).await.unwrap();
        let domains: Vec<&str> = all.iter().map(|d| d.domain.as_str()).collect();
        assert!(domains.contains(&"a.com"));
        assert!(domains.contains(&"b.com"));
        assert!(domains.contains(&"d.com"));
        assert!(!domains.contains(&"c.com"));
    }
}
