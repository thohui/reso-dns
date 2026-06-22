use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use axum_extra::extract::CookieJar;

use crate::{
    api::{
        auth::AllowedAuthMethods,
        cookie::{SESSION_COOKIE_KEY, decrypt_session_cookie},
        error::ApiError,
    },
    database::models::{api_key::ApiKey as DbApiKey, user::User, user_session::UserSession},
    global::SharedGlobal,
    utils::uuid::EntityId,
};

pub async fn auth_middleware(
    State((global, allowed)): State<(SharedGlobal, AllowedAuthMethods)>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if allowed.contains(AllowedAuthMethods::Session) {
        let cookie_value = CookieJar::from_headers(req.headers())
            .get(SESSION_COOKIE_KEY)
            .map(|c| c.value().to_string());

        if let Some(value) = cookie_value {
            match try_session_auth(&global, value).await {
                Ok((session_id, user_id)) => {
                    req.extensions_mut().insert(session_id);
                    req.extensions_mut().insert(user_id);
                    return Ok(next.run(req).await);
                }
                Err(e) => return Err(e),
            }
        }
    }

    if allowed.contains(AllowedAuthMethods::ApiKey) {
        let bearer = req
            .headers()
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|s| s.to_string());

        if let Some(bearer) = bearer {
            match try_api_key_auth(&global, bearer).await {
                Ok(key) => {
                    req.extensions_mut().insert(key);
                    return Ok(next.run(req).await);
                }
                Err(e) => return Err(e),
            }
        }
    }

    Err(ApiError::authentication_required())
}

async fn try_session_auth(
    global: &SharedGlobal,
    cookie_value: String,
) -> Result<(EntityId<UserSession>, EntityId<User>), ApiError> {
    let session_id =
        decrypt_session_cookie(&global.cipher, &cookie_value).map_err(|_| ApiError::invalid_credentials())?;
    let user_id = global.auth.verify_session(session_id.clone()).await?;
    Ok((session_id, user_id))
}

async fn try_api_key_auth(global: &SharedGlobal, bearer: String) -> Result<EntityId<DbApiKey>, ApiError> {
    let id = global.api_keys.verify_api_key(&bearer).await?;
    Ok(id)
}
