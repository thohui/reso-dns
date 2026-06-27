use rusqlite::{OptionalExtension, params};
use uuid::Uuid;

use crate::{
    database::{CoreDatabasePool, DatabaseError},
    time::now_millis,
    uuid::EntityId,
};

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
const INACTIVE_SESSION_TIMEOUT: i64 = 7 * 24 * 60 * 60 * 1000;

impl UserSession {
    pub fn new(user_id: EntityId<User>) -> Self {
        let now = now_millis();
        Self {
            id: EntityId::from(Uuid::now_v7()),
            user_id,
            created_at: now,
            expires_at: now + INACTIVE_SESSION_TIMEOUT,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = now_millis();
        now > self.expires_at
    }
}
pub async fn insert(db: &CoreDatabasePool, user_session: UserSession) -> Result<(), DatabaseError> {
    db.interact(move |c| {
        c.execute(
            r#"
					INSERT INTO user_sessions
						(id, user_id, created_at, expires_at)
					VALUES (?1, ?2, ?3, ?4)
					"#,
            params![
                user_session.id.id(),
                user_session.user_id.id(),
                user_session.created_at,
                user_session.expires_at
            ],
        )?;
        Ok(())
    })
    .await?;

    Ok(())
}

pub async fn find_by_id(
    db: &CoreDatabasePool,
    id: EntityId<UserSession>,
) -> Result<Option<UserSession>, DatabaseError> {
    db.interact(move |c| {
        c.query_row(
            "SELECT id, user_id,  created_at, expires_at FROM user_sessions WHERE id = ?1",
            params![id.id()],
            |f| {
                let session_id: Uuid = f.get(0)?;
                let user_id: Uuid = f.get(1)?;
                Ok(UserSession {
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
}

pub async fn delete_by_id(
    db: &CoreDatabasePool,
    session_id: EntityId<UserSession>,
) -> Result<bool, DatabaseError> {
    let rows = db
        .interact(move |c| c.execute("DELETE FROM user_sessions where id = ?", params![session_id.id()]))
        .await?;
    Ok(rows > 0)
}
