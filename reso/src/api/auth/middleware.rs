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
    database::models::{api_key::ApiKey as DbApiKey, user_session::UserSession},
    global::SharedGlobal,
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
                Ok(session) => {
                    req.extensions_mut().insert(session);
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

async fn try_session_auth(global: &SharedGlobal, cookie_value: String) -> Result<UserSession, ApiError> {
    let id = decrypt_session_cookie(&global.cipher, &cookie_value).map_err(|_| ApiError::invalid_credentials())?;

    let session = match UserSession::find_by_id(&global.core_database, id).await {
        Ok(Some(s)) => s,
        Ok(None) => return Err(ApiError::invalid_credentials()),
        Err(e) => {
            tracing::error!("failed to find user session: {:?}", e);
            return Err(ApiError::invalid_credentials());
        }
    };

    if session.is_expired() {
        if let Err(e) = session.delete(&global.core_database).await {
            tracing::error!("failed to delete expired session: {:?}", e);
        }
        return Err(ApiError::session_expired());
    }

    Ok(session)
}

async fn try_api_key_auth(global: &SharedGlobal, bearer: String) -> Result<DbApiKey, ApiError> {
    let hash = DbApiKey::hash_token(&bearer);

    let key = match DbApiKey::get_by_hash(&global.core_database, hash).await {
        Ok(Some(k)) => k,
        Ok(None) => return Err(ApiError::invalid_credentials()),
        Err(e) => {
            tracing::error!("failed to look up api key: {:?}", e);
            return Err(ApiError::invalid_credentials());
        }
    };

    if key.is_expired() {
        return Err(ApiError::invalid_credentials());
    }

    Ok(key)
}
