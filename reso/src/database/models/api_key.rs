use rand::RngCore;
use rusqlite::{OptionalExtension, params, types::Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    database::{
        CoreDatabasePool, DatabaseError,
        models::{Page, user::User},
        query::WhereBuilder,
    },
    time::now_millis,
    uuid::EntityId,
};

/// An API key that can be used to authenticate API requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiKey {
    pub id: EntityId<Self>,
    pub display_name: String,
    pub user_id: EntityId<User>,
    pub key_hash: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

const API_KEY_PREFIX: &str = "reso_";

fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("{API_KEY_PREFIX}{hex}")
}

impl ApiKey {
    pub fn hash_token(token: &str) -> String {
        let hash = Sha256::digest(token.as_bytes());
        hash.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Create a new API key.
    /// Returns a tuple of (ApiKey, raw_token). The raw token is never returned again.
    pub fn new(display_name: String, user_id: EntityId<User>, expires_at: Option<i64>) -> (Self, String) {
        let token = generate_token();
        let key_hash = Self::hash_token(&token);
        let key = Self {
            id: EntityId::new(),
            display_name,
            user_id,
            key_hash,
            created_at: now_millis(),
            expires_at,
        };
        (key, token)
    }

    /// Find an API key by its hash.
    pub async fn find_by_hash(db: &CoreDatabasePool, hash: String) -> Result<Option<Self>, DatabaseError> {
        db.interact(move |c| {
            c.query_row(
                "SELECT id, display_name, user_id, key_hash, created_at, expires_at FROM api_keys WHERE key_hash = ?1",
                params![hash],
                |r| {
                    Ok(Self {
                        id: EntityId::from(r.get::<_, Uuid>(0)?),
                        display_name: r.get(1)?,
                        user_id: EntityId::from(r.get::<_, Uuid>(2)?),
                        key_hash: r.get(3)?,
                        created_at: r.get(4)?,
                        expires_at: r.get(5)?,
                    })
                },
            )
            .optional()
        })
        .await
    }

    /// Insert the API key into the database.
    pub async fn insert(self, db: &CoreDatabasePool) -> Result<(), DatabaseError> {
        db.interact(move |c| {
            c.execute(
                "INSERT INTO api_keys (id, display_name, user_id, key_hash, created_at, expires_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    self.id.id(),
                    self.display_name,
                    self.user_id.id(),
                    self.key_hash,
                    self.created_at,
                    self.expires_at
                ],
            )?;
            Ok(())
        })
        .await?;
        Ok(())
    }

    /// List API keys joined with their owner's username.
    pub async fn list_with_username(
        db: &CoreDatabasePool,
        limit: i64,
        offset: i64,
        search: Option<String>,
    ) -> Result<Page<(Self, String)>, DatabaseError> {
        db.interact(move |c| {
            let mut count_b = WhereBuilder::new(0);
            if let Some(ref s) = search {
                count_b.like("display_name", s);
            }
            let (count_where, count_params) = count_b.build();
            let count_sql = format!("SELECT COUNT(*) FROM api_keys WHERE 1=1 {count_where}");
            let total = c.query_row(&count_sql, rusqlite::params_from_iter(&count_params), |r| r.get(0))?;

            let mut list_b = WhereBuilder::new(2);
            if let Some(ref s) = search {
                list_b.like("ak.display_name", s);
            }
            let (list_where, list_filter_params) = list_b.build();
            let list_sql = format!(
                "SELECT ak.id, ak.display_name, ak.user_id, ak.key_hash, ak.created_at, ak.expires_at, u.name
                     FROM api_keys ak
                     JOIN users u ON u.id = ak.user_id
                     WHERE 1=1 {list_where}
                     ORDER BY ak.created_at DESC, ak.id DESC
                     LIMIT ?1 OFFSET ?2"
            );
            let mut list_params: Vec<Value> = vec![Value::Integer(limit), Value::Integer(offset)];
            list_params.extend(list_filter_params);

            let mut stmt = c.prepare(&list_sql)?;
            let items = stmt
                .query_map(rusqlite::params_from_iter(&list_params), |r| {
                    Ok((
                        Self {
                            id: EntityId::from(r.get::<_, Uuid>(0)?),
                            display_name: r.get(1)?,
                            user_id: EntityId::from(r.get::<_, Uuid>(2)?),
                            key_hash: r.get(3)?,
                            created_at: r.get(4)?,
                            expires_at: r.get(5)?,
                        },
                        r.get::<_, String>(6)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(Page {
                items,
                total: Some(total),
            })
        })
        .await
    }

    /// Delete an API key by its ID.
    pub async fn delete_by_id(db: &CoreDatabasePool, id: &EntityId<Self>) -> Result<bool, DatabaseError> {
        let id = *id.id();
        let changed = db
            .interact(move |c| c.execute("DELETE FROM api_keys WHERE id = ?1", params![id]))
            .await?;
        Ok(changed > 0)
    }

    /// Check if the API key is expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|expires_at| now_millis() >= expires_at)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::models::user::User;
    use crate::database::setup_core_test_db;

    async fn insert_test_user(db: &CoreDatabasePool) -> EntityId<User> {
        let user = User::new("testuser", "hash");
        let id = user.id.clone();
        user.insert(db).await.unwrap();
        id
    }

    #[tokio::test]
    async fn test_insert_and_list() {
        let db = setup_core_test_db().await.unwrap();
        let user_id = insert_test_user(&db.conn).await;
        let expires_at = now_millis() + 60_000;

        let (key1, _) = ApiKey::new("test token".into(), user_id.clone(), None);
        key1.insert(&db.conn).await.unwrap();

        let (key2, _) = ApiKey::new("another token".into(), user_id.clone(), Some(expires_at));
        key2.insert(&db.conn).await.unwrap();

        let page = ApiKey::list_with_username(&db.conn, 10, 0, None).await.unwrap();
        assert_eq!(page.items.len(), 2);
        assert!(page.items.iter().any(|(k, _)| k.expires_at.is_none()));
        assert!(page.items.iter().any(|(k, _)| k.expires_at == Some(expires_at)));
    }

    #[tokio::test]
    async fn test_find_by_hash() {
        let db = setup_core_test_db().await.unwrap();
        let user_id = insert_test_user(&db.conn).await;

        let (key, token) = ApiKey::new("test token".into(), user_id, None);
        key.insert(&db.conn).await.unwrap();

        let found = ApiKey::find_by_hash(&db.conn, ApiKey::hash_token(&token))
            .await
            .unwrap();
        assert!(found.is_some());

        let not_found = ApiKey::find_by_hash(&db.conn, "notahash".to_string()).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_delete() {
        let db = setup_core_test_db().await.unwrap();
        let user_id = insert_test_user(&db.conn).await;

        let (key, _) = ApiKey::new("test token".into(), user_id, None);
        let key_id = key.id.clone();
        key.insert(&db.conn).await.unwrap();

        ApiKey::delete_by_id(&db.conn, &key_id).await.unwrap();

        assert!(
            ApiKey::list_with_username(&db.conn, 10, 0, None)
                .await
                .unwrap()
                .items
                .is_empty()
        );
    }

    #[tokio::test]
    async fn test_is_expired() {
        let (no_expiry, _) = ApiKey::new("test token".into(), EntityId::<User>::new(), None);
        assert!(!no_expiry.is_expired());

        let (future, _) = ApiKey::new(
            "test token".into(),
            EntityId::<User>::new(),
            Some(now_millis() + 60_000),
        );
        assert!(!future.is_expired());

        let (past, _) = ApiKey::new("test token".into(), EntityId::<User>::new(), Some(now_millis() - 1));
        assert!(past.is_expired());
    }
}
