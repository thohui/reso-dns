use rusqlite::{OptionalExtension, params};
use uuid::Uuid;

use crate::{
    database::{CoreDatabasePool, DatabaseError},
    time::now_millis,
    uuid::EntityId,
};

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
        let created_at = now_millis();
        User {
            id: EntityId::new(),
            name: name.into(),
            password_hash: password_hash.into(),
            created_at,
        }
    }
}

pub async fn insert(db: &CoreDatabasePool, user: User) -> Result<(), DatabaseError> {
    db.interact(move |c| {
        c.execute(
            "
					INSERT INTO users (id, name, password_hash, created_at)
					VALUES (?1, ?2, ?3, ?4)
					",
            params![user.id.id(), user.name, user.password_hash, user.created_at],
        )?;
        Ok(())
    })
    .await?;
    Ok(())
}

pub async fn find_by_name(db: &CoreDatabasePool, name: impl Into<String>) -> Result<Option<User>, DatabaseError> {
    let name = name.into();

    db.interact(move |c| {
        c.query_row(
            "SELECT id, name, password_hash, created_at FROM users WHERE name = ?1",
            params![name],
            |f| {
                let uuid: Uuid = f.get(0)?;
                Ok(User {
                    id: EntityId::from(uuid),
                    name: f.get(1)?,
                    password_hash: f.get(2)?,
                    created_at: f.get(3)?,
                })
            },
        )
        .optional()
    })
    .await
}

pub async fn find_by_id(db: &CoreDatabasePool, id: &EntityId<User>) -> Result<Option<User>, DatabaseError> {
    let id = *id.id();

    db.interact(move |c| {
        c.query_row(
            "SELECT id, name, password_hash, created_at FROM users WHERE id = ?1",
            params![id],
            |f| {
                Ok(User {
                    id: EntityId::from(f.get::<usize, Uuid>(0)?),
                    name: f.get(1)?,
                    password_hash: f.get(2)?,
                    created_at: f.get(3)?,
                })
            },
        )
        .optional()
    })
    .await
}

pub async fn count(db: &CoreDatabasePool) -> Result<i64, DatabaseError> {
    db.interact(|c| c.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0)))
        .await
}

#[allow(unused)]
pub async fn list(db: &CoreDatabasePool) -> Result<Vec<User>, DatabaseError> {
    db.interact(|c| {
        let mut stmt = c.prepare("SELECT id, name, password_hash, created_at FROM users")?;
        let iter = stmt.query_map([], |r| {
            Ok(User {
                id: EntityId::from(r.get::<_, Uuid>(0)?),
                name: r.get(1)?,
                password_hash: r.get(2)?,
                created_at: r.get(3)?,
            })
        })?;
        iter.collect::<rusqlite::Result<Vec<_>>>()
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::setup_core_test_db;

    #[tokio::test]
    async fn test_user_new() {
        let user = User::new("testuser", "hash123");

        assert_eq!(user.name, "testuser");
        assert_eq!(user.password_hash, "hash123");
        assert!(user.created_at > 0);
    }

    #[tokio::test]
    async fn test_user_insert_and_find_by_name() {
        let db = setup_core_test_db().await.unwrap();

        let user = User::new("alice", "password_hash_alice");

        let created_at = user.created_at;
        insert(&db.conn, user).await.unwrap();

        let found = find_by_name(&db.conn, "alice").await.unwrap();
        assert!(found.is_some());

        let found_user = found.unwrap();

        assert_eq!(found_user.name, "alice");
        assert_eq!(found_user.password_hash, "password_hash_alice");
        assert_eq!(found_user.created_at, created_at);
    }

    #[tokio::test]
    async fn test_user_insert_and_find_by_id() {
        let db = setup_core_test_db().await.unwrap();
        let user = User::new("bob", "password_hash_bob");
        let user_id = user.id.clone();

        insert(&db.conn, user).await.unwrap();

        let found = find_by_id(&db.conn, &user_id).await.unwrap();
        assert!(found.is_some());

        let found_user = found.unwrap();
        assert_eq!(found_user.id, user_id);
        assert_eq!(found_user.name, "bob");
        assert_eq!(found_user.password_hash, "password_hash_bob");
    }

    #[tokio::test]
    async fn test_user_list_multiple() {
        let db = setup_core_test_db().await.unwrap();

        let user1 = User::new("user1", "hash1");
        let user2 = User::new("user2", "hash2");
        let user3 = User::new("user3", "hash3");

        insert(&db.conn, user1).await.unwrap();
        insert(&db.conn, user2).await.unwrap();
        insert(&db.conn, user3).await.unwrap();

        let users = list(&db.conn).await.unwrap();
        assert_eq!(users.len(), 3);

        let names: Vec<String> = users.iter().map(|u| u.name.clone()).collect();
        assert!(names.contains(&"user1".to_string()));
        assert!(names.contains(&"user2".to_string()));
        assert!(names.contains(&"user3".to_string()));
    }
}
