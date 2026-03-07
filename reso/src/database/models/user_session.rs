use anyhow::Context;
use chrono::Utc;
use rusqlite::{OptionalExtension, params};
use uuid::Uuid;

use crate::{database::CoreDatabasePool, utils::uuid::EntityId};

use super::user::User;

#[derive(Clone)]
pub struct UserSession {
    pub id: EntityId<Self>,
    pub user_id: EntityId<User>,
    /// Time in ms.
    pub created_at: i64,
    /// Time in ms.
    pub expires_at: i64,
}

/// The amount of days that a session can be inactive before it is considered expired.
const INACTIVE_SESSION_TIMEOUT: i64 = chrono::Duration::days(7).num_milliseconds();

/// The amount of days that must pass before a session's last active time is updated.
pub const UPDATE_THRESHOLD: i64 = chrono::Duration::days(1).num_milliseconds();

impl UserSession {
    pub fn new(user_id: EntityId<User>) -> Self {
        let now = Utc::now().timestamp_millis();

        Self {
            id: EntityId::from(Uuid::now_v7()),
            user_id,
            created_at: now,
            expires_at: now + INACTIVE_SESSION_TIMEOUT,
        }
    }

    pub async fn insert(self, db: &CoreDatabasePool) -> anyhow::Result<()> {
        db.interact(move |c| {
            c.execute(
                r#"
					INSERT INTO user_sessions
						(id, user_id, created_at, expires_at)
					VALUES (?1, ?2, ?3, ?4)
					"#,
                params![self.id.id(), self.user_id.id(), self.created_at, self.expires_at],
            )?;
            Ok(())
        })
        .await
        .context("failed to insert user session")?;

        Ok(())
    }

    pub async fn find_by_id(db: &CoreDatabasePool, id: EntityId<Self>) -> anyhow::Result<Option<Self>> {
        Ok(db
            .interact(move |c| {
                c.query_row(
                    "SELECT id, user_id,  created_at, expires_at FROM user_sessions WHERE id = ?1",
                    params![id.id()],
                    |f| {
                        let session_id: Uuid = f.get(0)?;
                        let user_id: Uuid = f.get(1)?;
                        Ok(Self {
                            id: EntityId::from(session_id),
                            user_id: EntityId::from(user_id),
                            created_at: f.get(2)?,
                            expires_at: f.get(3)?,
                        })
                    },
                )
                .optional()
            })
            .await
            .context("failed to find user session by id")?)
    }

    pub async fn delete(self, db: &CoreDatabasePool) -> anyhow::Result<()> {
        db.interact(move |c| {
            c.execute("DELETE FROM user_sessions where id = ?", params![self.id.id()])?;
            Ok(())
        })
        .await?;
        Ok(())
    }

    pub async fn delete_by_user_id(db: &CoreDatabasePool, user_id: EntityId<User>) -> anyhow::Result<()> {
        db.interact(move |c| {
            c.execute("DELETE FROM user_sessions where user_id = ?", params![user_id.id()])?;
            Ok(())
        })
        .await
        .context("failed to delete user session")?;
        Ok(())
    }

    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp_millis();
        now.saturating_sub(self.expires_at) > INACTIVE_SESSION_TIMEOUT
    }
}
