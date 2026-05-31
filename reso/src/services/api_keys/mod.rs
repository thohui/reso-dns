use std::sync::Arc;

use crate::{
    database::{
        CoreDatabasePool,
        models::{Page, api_key::ApiKey as DbApiKey, user::User as DbUser},
    },
    services::ServiceError,
    utils::uuid::EntityId,
};

pub struct ApiKeysService {
    db: Arc<CoreDatabasePool>,
}

/// Service representation of an API key (for listing).
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
        let (api_key, token) = DbApiKey::new(display_name, user_id, expires_at);

        let id = api_key.id.clone();
        let db_user_id = api_key.user_id.clone();
        let created_at = api_key.created_at;
        let expires_at = api_key.expires_at;
        let display_name = api_key.display_name.clone();

        api_key.insert(&self.db).await?;

        let username = DbUser::find_by_id(&self.db, &db_user_id)
            .await?
            .ok_or_else(|| ServiceError::Internal(anyhow::anyhow!("user not found")))?
            .name;

        Ok(CreatedApiKey {
            id,
            display_name,
            created_by: username,
            created_at,
            expires_at,
            token,
        })
    }

    /// List all API keys, with pagination.
    pub async fn list_api_keys(&self, limit: i64, offset: i64) -> Result<Page<ApiKey>, ServiceError> {
        let page = DbApiKey::list_with_username(&self.db, limit, offset).await?;
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

    /// Delete an API key by its id.
    pub async fn delete_api_key(&self, id: &EntityId<DbApiKey>) -> Result<(), ServiceError> {
        let changed = DbApiKey::delete(&self.db, id).await?;
        if changed {
            Ok(())
        } else {
            Err(ServiceError::NotFound("API key not found".into()))
        }
    }
}
