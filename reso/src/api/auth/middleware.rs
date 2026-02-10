use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
    response::Result,
};
use axum_extra::extract::CookieJar;

use crate::{
    api::{
        cookie::{SESSION_COOKIE_KEY, decrypt_session_cookie},
        error::ApiError,
    },
    database::models::user_session::UserSession,
    global::SharedGlobal,
};

pub async fn auth_middleware(global: State<SharedGlobal>, mut req: Request, next: Next) -> Result<Response, ApiError> {
    let cookie_jar = CookieJar::from_headers(req.headers());

    let cookie = if let Some(value) = cookie_jar.get(SESSION_COOKIE_KEY) {
        value
    } else {
        return Err(ApiError::authentication_required());
    };

    let value = cookie.value();

    let id = if let Ok(id) = decrypt_session_cookie(&global.cipher, value) {
        id
    } else {
        return Err(ApiError::invalid_credentials());
    };

    let session = if let Ok(session) = UserSession::find_by_id(&global.database, id).await {
        session
    } else {
        return Err(ApiError::invalid_credentials());
    };

    if session.is_expired() {
        if let Err(e) = session.delete(&global.database).await {
            tracing::error!("failed to delete user session {:?}", e);
        }
        return Err(ApiError::session_expired());
    };

    req.extensions_mut().insert(session);

    Ok(next.run(req).await)
}
