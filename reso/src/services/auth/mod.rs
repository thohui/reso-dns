use std::sync::Arc;

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};

use crate::{
    database::{
        CoreDatabasePool,
        models::{user::User, user_session::UserSession as DbUserSession},
    },
    services::ServiceError,
    utils::uuid::EntityId,
};

pub struct AuthService {
    db: Arc<CoreDatabasePool>,
}

impl AuthService {
    pub fn new(db: Arc<CoreDatabasePool>) -> Self {
        Self { db }
    }

    /// Performs initial server setup by creating the first admin user and returning an authenticated session.
    pub async fn setup(&self, username: &str, password: &str) -> Result<EntityId<DbUserSession>, ServiceError> {
        let count = User::count(&self.db).await?;

        if count > 0 {
            return Err(ServiceError::Conflict("setup already completed".into()));
        }

        if username.trim().is_empty() || password.len() < 8 {
            return Err(ServiceError::BadRequest("invalid credentials".into()));
        }

        let hash = hash_password(password)?;

        let user = User::new(username.trim(), hash);
        let user_id = user.id.clone();
        user.insert(&self.db).await?;

        self.create_session(user_id).await
    }

    /// Verify credentials and return a session id.
    pub async fn login(&self, username: &str, password: &str) -> Result<EntityId<DbUserSession>, ServiceError> {
        let user = match User::find_by_name(&self.db, username).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                // Simulate hashing to prevent timing attacks.
                let _ = hash_password(password);
                return Err(ServiceError::Unauthorized("invalid credentials".into()));
            }
            Err(e) => {
                let _ = hash_password(password);
                return Err(ServiceError::Internal(e.into()));
            }
        };

        verify_password(password, &user.password_hash)
            .map_err(|_| ServiceError::Unauthorized("invalid credentials".into()))?;

        self.create_session(user.id).await
    }

    /// Delete a session.
    pub async fn logout(&self, id: EntityId<DbUserSession>) -> Result<(), ServiceError> {
        let session = DbUserSession::find_by_id(&self.db, id)
            .await?
            .ok_or(ServiceError::Unauthorized("session not found".into()))?;

        session.delete(&self.db).await?;
        Ok(())
    }

    /// Validate a session id. Returns the user id on success.
    pub async fn verify_session(&self, id: EntityId<DbUserSession>) -> Result<EntityId<User>, ServiceError> {
        let session = DbUserSession::find_by_id(&self.db, id)
            .await?
            .ok_or(ServiceError::Unauthorized("session not found".into()))?;

        if session.is_expired() {
            let _ = session.delete(&self.db).await;
            return Err(ServiceError::Unauthorized("session expired".into()));
        }

        Ok(session.user_id)
    }

    /// Return the number of registered users.
    pub async fn user_count(&self) -> Result<i64, ServiceError> {
        Ok(User::count(&self.db).await?)
    }

    async fn create_session(&self, user_id: EntityId<User>) -> Result<EntityId<DbUserSession>, ServiceError> {
        let session = DbUserSession::new(user_id);
        let id = session.id.clone();
        session.insert(&self.db).await?;
        Ok(id)
    }
}

fn hash_password(password: &str) -> Result<String, ServiceError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ServiceError::Internal(e.into()))
}

fn verify_password(password: &str, hash: &str) -> Result<(), ServiceError> {
    let hash = PasswordHash::new(hash).map_err(|e| ServiceError::Internal(e.into()))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &hash)
        .map_err(|e| ServiceError::Internal(e.into()))
}
