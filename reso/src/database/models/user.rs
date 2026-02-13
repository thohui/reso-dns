use chrono::Utc;
use tokio_rusqlite::{OptionalExtension, params, rusqlite};
use uuid::Uuid;

use crate::{database::DatabaseConnection, utils::uuid::EntityId};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct User {
    pub id: EntityId<Self>,
    pub name: String,
    pub password_hash: String,
    /// Time in ms.
    pub created_at: i64,
}

impl User {
    pub fn new(name: impl Into<String>, password_hash: impl Into<String>) -> Self {
        let created_at = Utc::now().timestamp_millis();
        User {
            id: EntityId::new(),
            name: name.into(),
            password_hash: password_hash.into(),
            created_at,
        }
    }

    pub async fn insert(self, db: &DatabaseConnection) -> anyhow::Result<()> {
        let conn = db.conn().await;

        conn.call(move |c| -> rusqlite::Result<()> {
            c.execute(
                r#"
					INSERT INTO users
						(id, name, password_hash, created_at) 
					VALUES (?1, ?2, ?3, ?4)
					"#,
                params![self.id.id(), self.name, self.password_hash, self.created_at],
            )?;
            Ok(())
        })
        .await?;

        Ok(())
    }

    pub async fn find_by_name(db: &DatabaseConnection, name: impl Into<String>) -> anyhow::Result<Option<Self>> {
        let conn = db.conn().await;

        let name = name.into();

        let user = conn
            .call(move |c| {
                c.query_one(
                    "SELECT id, name, password_hash, created_at FROM users WHERE name = ?1",
                    params![name],
                    |f| {
                        let uuid: Uuid = f.get(0)?;
                        Ok(Self {
                            id: EntityId::from(uuid),
                            name: f.get(1)?,
                            password_hash: f.get(2)?,
                            created_at: f.get(3)?,
                        })
                    },
                )
                .optional()
            })
            .await?;
        Ok(user)
    }

    pub async fn find_by_id(db: &DatabaseConnection, id: &EntityId<Self>) -> anyhow::Result<Option<Self>> {
        let conn = db.conn().await;

        let id = id.id().clone();

        let user = conn
            .call(move |c| {
                c.query_one(
                    "SELECT id, name, password_hash, created_at FROM users WHERE id = ?1",
                    params![id],
                    |f| {
                        Ok(Self {
                            id: EntityId::from(f.get::<usize, Uuid>(0)?),
                            name: f.get(1)?,
                            password_hash: f.get(2)?,
                            created_at: f.get(3)?,
                        })
                    },
                )
                .optional()
            })
            .await?;
        Ok(user)
    }

    pub async fn list(db: &DatabaseConnection) -> anyhow::Result<Vec<Self>> {
        let conn = db.conn().await;

        let raw: Vec<Self> = conn
            .call(|c| {
                let mut stmt = c.prepare("SELECT id, name, password_hash, created_at FROM users")?;
                let iter = stmt.query_map([], |r| {
                    Ok(Self {
                        id: EntityId::from(r.get::<usize, Uuid>(0)?),
                        name: r.get(1)?,
                        password_hash: r.get(2)?,
                        created_at: r.get(3)?,
                    })
                })?;
                iter.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await?;

        Ok(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{connect, run_migrations, setup_test_db};
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_user_new() {
        let user = User::new("testuser", "hash123");

        assert_eq!(user.name, "testuser");
        assert_eq!(user.password_hash, "hash123");
        assert!(user.created_at > 0);
    }

    #[tokio::test]
    async fn test_user_insert_and_find_by_name() {
        let db = setup_test_db().await.unwrap();

        let user = User::new("alice", "password_hash_alice");

        user.clone().insert(&db).await.unwrap();

        let found = User::find_by_name(&db, "alice").await.unwrap();
        assert!(found.is_some());

        let found_user = found.unwrap();

        assert_eq!(found_user.name, "alice");
        assert_eq!(found_user.password_hash, "password_hash_alice");
        assert_eq!(found_user.created_at, user.created_at);
    }

    #[tokio::test]
    async fn test_user_find_by_name_not_found() {
        let db = setup_test_db().await.unwrap();

        let result = User::find_by_name(&db, "nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_user_insert_and_find_by_id() {
        let db = setup_test_db().await.unwrap();
        let user = User::new("bob", "password_hash_bob");
        let user_id = user.id.clone();

        user.insert(&db).await.unwrap();

        let found = User::find_by_id(&db, &user_id).await.unwrap();
        assert!(found.is_some());

        let found_user = found.unwrap();
        assert_eq!(found_user.id, user_id);
        assert_eq!(found_user.name, "bob");
        assert_eq!(found_user.password_hash, "password_hash_bob");
    }

    #[tokio::test]
    async fn test_user_find_by_id_not_found() {
        let db = setup_test_db().await.unwrap();
        let random_id = EntityId::<User>::new();

        let result = User::find_by_id(&db, &random_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_user_list_empty() {
        let db = setup_test_db().await.unwrap();

        let users = User::list(&db).await.unwrap();
        assert_eq!(users.len(), 0);
    }

    #[tokio::test]
    async fn test_user_list_multiple() {
        let db = setup_test_db().await.unwrap();

        let user1 = User::new("user1", "hash1");
        let user2 = User::new("user2", "hash2");
        let user3 = User::new("user3", "hash3");

        user1.insert(&db).await.unwrap();
        user2.insert(&db).await.unwrap();
        user3.insert(&db).await.unwrap();

        let users = User::list(&db).await.unwrap();
        assert_eq!(users.len(), 3);

        let names: Vec<String> = users.iter().map(|u| u.name.clone()).collect();
        assert!(names.contains(&"user1".to_string()));
        assert!(names.contains(&"user2".to_string()));
        assert!(names.contains(&"user3".to_string()));
    }

    #[tokio::test]
    async fn test_user_unique_ids() {
        let user1 = User::new("user1", "hash1");
        let user2 = User::new("user2", "hash2");

        assert_ne!(user1.id, user2.id);
    }

    #[tokio::test]
    async fn test_user_created_at_timestamp() {
        let before = chrono::Utc::now().timestamp_millis();
        let user = User::new("testuser", "hash");
        let after = chrono::Utc::now().timestamp_millis();

        assert!(user.created_at >= before);
        assert!(user.created_at <= after);
    }

    #[tokio::test]
    async fn test_user_password_hash_stored() {
        let db = setup_test_db().await.unwrap();
        let password_hash = "very_secure_hash_123";
        let user = User::new("secure_user", password_hash);

        user.insert(&db).await.unwrap();

        let found = User::find_by_name(&db, "secure_user").await.unwrap().unwrap();
        assert_eq!(found.password_hash, password_hash);
    }

    #[tokio::test]
    async fn test_user_with_empty_password_hash() {
        let db = setup_test_db().await.unwrap();
        let user = User::new("user_empty_hash", "");

        user.insert(&db).await.unwrap();

        let found = User::find_by_name(&db, "user_empty_hash").await.unwrap().unwrap();
        assert_eq!(found.password_hash, "");
    }

    #[tokio::test]
    async fn test_user_with_special_characters_in_name() {
        let db = setup_test_db().await.unwrap();
        let user = User::new("user@example.com", "hash");

        user.insert(&db).await.unwrap();

        let found = User::find_by_name(&db, "user@example.com").await.unwrap().unwrap();
        assert_eq!(found.name, "user@example.com");
    }

    #[tokio::test]
    async fn test_user_entity_id_conversion() {
        let user = User::new("test", "hash");
        let uuid = user.id.id().clone();

        let new_entity_id = EntityId::<User>::from(uuid);
        assert_eq!(user.id, new_entity_id);
    }
}
