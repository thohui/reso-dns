use std::sync::Arc;

use crate::{
    database::{
        CoreDatabasePool,
        models::{Page, api_key::ApiKey as DbApiKey, user::User as DbUser},
    },
    services::ServiceError,
    uuid::EntityId,
};

/// Service for managing API keys.
pub struct ApiKeysService {
    db: Arc<CoreDatabasePool>,
}

/// Service representation of an API key.
pub struct ApiKey {
    pub id: EntityId<DbApiKey>,
    pub display_name: String,
    pub created_by: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

/// Service representation of a created API key, which includes the raw token.
pub struct CreatedApiKey {
    pub id: EntityId<DbApiKey>,
    pub display_name: String,
    pub created_by: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub token: String,
}

impl ApiKeysService {
    pub fn new(db: Arc<CoreDatabasePool>) -> Self {
        Self { db }
    }

    /// Create a new API key for the given user.
    pub async fn create_api_key(
        &self,
        display_name: String,
        user_id: EntityId<DbUser>,
        expires_at: Option<i64>,
    ) -> Result<CreatedApiKey, ServiceError> {
        let username = DbUser::find_by_id(&self.db, &user_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound("user not found".to_owned()))?
            .name;

        let (api_key, token) = DbApiKey::new(display_name, user_id, expires_at);

        let id = api_key.id.clone();
        let created_at = api_key.created_at;
        let expires_at = api_key.expires_at;
        let display_name = api_key.display_name.clone();

        api_key.insert(&self.db).await?;

        Ok(CreatedApiKey {
            id,
            display_name,
            created_by: username,
            created_at,
            expires_at,
            token,
        })
    }

    /// List all API keys, with pagination and optional search.
    pub async fn list_api_keys(
        &self,
        limit: i64,
        offset: i64,
        search: Option<String>,
    ) -> Result<Page<ApiKey>, ServiceError> {
        let page = DbApiKey::list_with_username(&self.db, limit, offset, search).await?;
        Ok(Page {
            total: page.total,
            items: page
                .items
                .into_iter()
                .map(|(key, username)| ApiKey {
                    id: key.id,
                    display_name: key.display_name,
                    created_by: username,
                    created_at: key.created_at,
                    expires_at: key.expires_at,
                })
                .collect(),
        })
    }

    /// Validate an API key bearer token.
    pub async fn verify_api_key(&self, bearer: &str) -> Result<EntityId<DbApiKey>, ServiceError> {
        let hash = DbApiKey::hash_token(bearer);

        let key = DbApiKey::find_by_hash(&self.db, hash)
            .await?
            .ok_or(ServiceError::Unauthorized("invalid api key".into()))?;

        if key.is_expired() {
            return Err(ServiceError::Unauthorized("api key expired".into()));
        }

        Ok(key.id)
    }

    /// Delete an API key by its id.
    pub async fn delete_api_key(&self, id: &EntityId<DbApiKey>) -> Result<(), ServiceError> {
        let changed = DbApiKey::delete_by_id(&self.db, id).await?;
        if changed {
            Ok(())
        } else {
            Err(ServiceError::NotFound("API key not found".into()))
        }
    }
}
