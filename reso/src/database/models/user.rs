use chrono::Utc;
use tokio_rusqlite::{params, rusqlite};
use uuid::Uuid;

use crate::{database::DatabaseConnection, utils::uuid::EntityId};

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

    pub async fn find_by_name(db: &DatabaseConnection, name: impl Into<String>) -> anyhow::Result<Self> {
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
            })
            .await?;
        Ok(user)
    }

    pub async fn find_by_id(db: &DatabaseConnection, id: &EntityId<Self>) -> anyhow::Result<Self> {
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
            })
            .await?;
        Ok(user)
    }

    pub async fn list(db: &DatabaseConnection) -> anyhow::Result<Vec<Self>> {
        let conn = db.conn().await;

        let raw: Vec<Self> = conn
            .call(|c| -> rusqlite::Result<Vec<Self>> {
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
