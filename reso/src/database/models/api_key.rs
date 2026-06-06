use rand::RngCore;
use rusqlite::{OptionalExtension, params};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    database::{
        CoreDatabasePool, DatabaseError,
        models::{Page, user::User},
    },
    utils::{now_millis, uuid::EntityId},
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

    pub async fn get_by_id(db: &CoreDatabasePool, id: &EntityId<Self>) -> Result<Option<Self>, DatabaseError> {
        let id = *id.id();
        Ok(db
            .interact(move |c| {
                c.query_row(
                    "SELECT id, display_name, user_id, key_hash, created_at, expires_at FROM api_keys WHERE id = ?1",
                    params![id],
                    |r| {
                        Ok(Self {
                            id: EntityId::from(r.get::<_, Uuid>(0)?),
                            display_name: r.get(1)?,
                            user_id: EntityId::from(r.get::<_, Uuid>(2)?),
                            key_hash: r.get(2)?,
                            created_at: r.get(3)?,
                            expires_at: r.get(4)?,
                        })
                    },
                )
                .optional()
            })
            .await?)
    }

    pub async fn get_by_hash(db: &CoreDatabasePool, hash: String) -> Result<Option<Self>, DatabaseError> {
        Ok(db
            .interact(move |c| {
                c.query_row(
                    "SELECT id, user_id, key_hash, created_at, expires_at FROM api_keys WHERE key_hash = ?1",
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
            .await?)
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

    /// List API keys joined with their owner's username, with a total count.
    pub async fn list_with_username(
        db: &CoreDatabasePool,
        limit: i64,
        offset: i64,
    ) -> Result<Page<(Self, String)>, DatabaseError> {
        Ok(db
            .interact(move |c| {
                let total = c.query_row("SELECT COUNT(*) FROM api_keys", [], |r| r.get(0))?;
                let mut stmt = c.prepare(
                    "SELECT ak.id, ak.display_name, ak.user_id, ak.key_hash, ak.created_at, ak.expires_at, u.name
                     FROM api_keys ak
                     JOIN users u ON u.id = ak.user_id
                     LIMIT ?1 OFFSET ?2",
                )?;
                let items = stmt
                    .query_map(params![limit, offset], |r| {
                        Ok((
                            Self {
                                id: EntityId::from(r.get::<_, Uuid>(0)?),
                                display_name: r.get(1)?,
                                user_id: EntityId::from(r.get::<_, Uuid>(2)?),
                                key_hash: r.get(3)?,
                                created_at: r.get(4)?,
                                expires_at: r.get(5)?,
                            },
                            r.get::<_, String>(6)?, // username
                        ))
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(Page {
                    items,
                    total: Some(total),
                })
            })
            .await?)
    }

    /// Delete an API key by its ID.
    pub async fn delete(db: &CoreDatabasePool, id: &EntityId<Self>) -> Result<bool, DatabaseError> {
        let id = *id.id();
        let changed = db
            .interact(move |c| Ok(c.execute("DELETE FROM api_keys WHERE id = ?1", params![id])?))
            .await?;
        Ok(changed > 0)
    }

    /// Check if the API key is expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            now_millis() > expires_at
        } else {
            false
        }
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

        let page = ApiKey::list_with_username(&db.conn, 10, 0).await.unwrap();
        assert_eq!(page.items.len(), 2);
        assert!(page.items.iter().any(|(k, _)| k.expires_at.is_none()));
        assert!(page.items.iter().any(|(k, _)| k.expires_at == Some(expires_at)));
    }

    #[tokio::test]
    async fn test_get_by_hash() {
        let db = setup_core_test_db().await.unwrap();
        let user_id = insert_test_user(&db.conn).await;

        let (key, token) = ApiKey::new("test token".into(), user_id, None);
        key.insert(&db.conn).await.unwrap();

        let found = ApiKey::get_by_hash(&db.conn, ApiKey::hash_token(&token)).await.unwrap();
        assert!(found.is_some());

        let not_found = ApiKey::get_by_hash(&db.conn, "notahash".to_string()).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_delete() {
        let db = setup_core_test_db().await.unwrap();
        let user_id = insert_test_user(&db.conn).await;

        let (key, _) = ApiKey::new("test token".into(), user_id, None);
        let key_id = key.id.clone();
        key.insert(&db.conn).await.unwrap();

        ApiKey::delete(&db.conn, &key_id).await.unwrap();

        assert!(
            ApiKey::list_with_username(&db.conn, 10, 0)
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
