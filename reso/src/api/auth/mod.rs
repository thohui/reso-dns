use axum::{
    Extension, Json, Router,
    extract::State,
    http::StatusCode,
    middleware as axum_middleware,
    response::{IntoResponse, Response},
    routing::post,
};
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};

use crate::{database::models::user_session::UserSession, global::SharedGlobal, utils::uuid::EntityId};

use super::{cookie, error::ApiError};
use middleware::auth_middleware;

pub mod middleware;

bitflags::bitflags! {
    /// Allowed authentication methods for API routes.
    #[derive(Clone, Copy)]
    pub struct AllowedAuthMethods: u32 {
        /// Allow authentication via session cookie.
        const Session = 1 << 0;
        /// Allow authentication via API key.
        const ApiKey = 1 << 1;
    }
}

pub fn create_auth_router(global: SharedGlobal) -> Router<SharedGlobal> {
    let authenticated = Router::new()
        .route("/logout", post(logout))
        .layer(axum_middleware::from_fn_with_state(
            (global.clone(), AllowedAuthMethods::Session),
            auth_middleware,
        ));

    Router::new()
        .route("/login", post(login))
        .route("/check", post(check))
        .route("/setup", post(setup))
        .merge(authenticated)
        .with_state(global)
}

#[derive(Deserialize)]
pub(crate) struct LoginPayload {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct CheckResponse {
    authenticated: bool,
    setup_required: bool,
}

pub async fn setup(
    global: State<SharedGlobal>,
    jar: CookieJar,
    payload: Json<LoginPayload>,
) -> Result<Response, ApiError> {
    let session_id = global.auth.setup(&payload.username, &payload.password).await?;

    let encrypted = cookie::encrypt_session_id(&global.cipher, session_id).map_err(|_| ApiError::server_error())?;

    let jar = jar.add(cookie::build_session_cookie(encrypted));

    tracing::info!("setup completed: admin user '{}' created", payload.username);
    Ok((jar, StatusCode::OK).into_response())
}

pub async fn login(
    global: State<SharedGlobal>,
    jar: CookieJar,
    payload: Json<LoginPayload>,
) -> Result<Response, ApiError> {
    let session_id = global.auth.login(&payload.username, &payload.password).await?;

    let encrypted = cookie::encrypt_session_id(&global.cipher, session_id).map_err(|_| ApiError::server_error())?;

    let jar = jar.add(cookie::build_session_cookie(encrypted));
    Ok((jar, StatusCode::OK).into_response())
}

pub async fn logout(
    global: State<SharedGlobal>,
    Extension(session_id): Extension<EntityId<UserSession>>,
    jar: CookieJar,
) -> Result<Response, ApiError> {
    let jar = jar.remove(cookie::SESSION_COOKIE_KEY);

    global
        .auth
        .logout(session_id)
        .await
        .map_err(|_| ApiError::server_error().cookie_jar(jar.clone()))?;

    Ok((jar, StatusCode::OK).into_response())
}

pub async fn check(global: State<SharedGlobal>, jar: CookieJar) -> Result<Json<CheckResponse>, ApiError> {
    // TODO: this shouldn't be part of the auth service.
    let count = global.auth.user_count().await?;

    if count == 0 {
        return Ok(Json(CheckResponse {
            authenticated: false,
            setup_required: true,
        }));
    }

    let authenticated = async {
        let value = jar.get(cookie::SESSION_COOKIE_KEY)?.value().to_string();
        let session_id = cookie::decrypt_session_cookie(&global.cipher, &value).ok()?;
        global.auth.verify_session(session_id).await.ok()
    }
    .await
    .is_some();

    Ok(Json(CheckResponse {
        authenticated,
        setup_required: false,
    }))
}
