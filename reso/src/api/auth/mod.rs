use axum::{
    Extension, Json, Router,
    extract::State,
    http::StatusCode,
    middleware as axum_middleware,
    response::{IntoResponse, Response},
    routing::post,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;

use crate::{
    database::models::{user::User, user_session::UserSession},
    global::SharedGlobal,
    utils::password,
};

use super::{cookie, error::ApiError};
use middleware::auth_middleware;

pub mod middleware;

pub fn create_auth_router(global: SharedGlobal) -> Router<SharedGlobal> {
    Router::new()
        .route("/login", post(login))
        .route(
            "/logout",
            post(logout).layer(axum_middleware::from_fn_with_state(global.clone(), auth_middleware)),
        )
        .route(
            "/check",
            post(check).layer(axum_middleware::from_fn_with_state(global.clone(), auth_middleware)),
        )
        .with_state(global)
}

#[derive(Deserialize)]
pub(crate) struct LoginPayload {
    username: String,
    password: String,
}

pub async fn login(
    global: State<SharedGlobal>,
    jar: CookieJar,
    payload: Json<LoginPayload>,
) -> axum::response::Result<Response, ApiError> {
    let user = match User::find_by_name(&global.database, payload.username.clone()).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            let _ = password::hash_password(&payload.password);
            return Err(ApiError::invalid_credentials());
        }
        Err(e) => {
            tracing::error!("failed to find user by name {:?}", e);
            // Simulate a slow response to prevent timing attacks.
            let _ = password::hash_password(&payload.password);
            return Err(ApiError::invalid_credentials());
        }
    };

    if password::verify_password(&payload.password, &user.password_hash).is_err() {
        return Err(ApiError::invalid_credentials());
    }

    let session = UserSession::new(user.id);
    let session_id = session.id.clone();

    session.insert(&global.database).await.map_err(|e| {
        tracing::error!("failed to insert user session: {:?}", e);
        ApiError::server_error()
    })?;

    let encrypted_cookie = cookie::encrypt_session_id(&global.cipher, session_id).map_err(|e| {
        tracing::error!("failed to encrypt the session id: {:?}", e);
        ApiError::server_error()
    })?;

    let c = cookie::build_session_cookie(encrypted_cookie);
    let jar = jar.add(c);

    Ok((jar, StatusCode::OK).into_response())
}

pub async fn logout(
    global: State<SharedGlobal>,
    Extension(session): Extension<UserSession>,
    jar: CookieJar,
) -> axum::response::Result<Response, ApiError> {
    let jar = jar.remove(cookie::SESSION_COOKIE_KEY);

    session
        .delete(&global.database)
        .await
        .map_err(|_| ApiError::server_error().cookie_jar(jar.clone()))?;

    Ok((jar, StatusCode::OK).into_response())
}

pub async fn check() -> axum::response::Result<Response, ApiError> {
    return Ok(StatusCode::OK.into_response());
}
