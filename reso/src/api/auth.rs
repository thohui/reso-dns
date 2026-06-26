use axum::{
    Extension, Json, Router,
    extract::{Request, State},
    http::StatusCode,
    middleware as axum_middleware,
    middleware::Next,
    response::{IntoResponse, Response},
    routing::post,
};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};

use crate::{
    database::models::{api_key::ApiKey as DbApiKey, user::User, user_session::UserSession},
    global::SharedGlobal,
    uuid::EntityId,
};

use super::{cookie, error::ApiError};

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
        .map_err(|_| ApiError::server_error().with_cookie_jar(jar.clone()))?;

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

pub async fn auth_middleware(
    State((global, allowed)): State<(SharedGlobal, AllowedAuthMethods)>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if allowed.contains(AllowedAuthMethods::Session) {
        let cookie_value = CookieJar::from_headers(req.headers())
            .get(cookie::SESSION_COOKIE_KEY)
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
        cookie::decrypt_session_cookie(&global.cipher, &cookie_value).map_err(|_| ApiError::invalid_credentials())?;
    let user_id = global.auth.verify_session(session_id.clone()).await?;
    Ok((session_id, user_id))
}

async fn try_api_key_auth(global: &SharedGlobal, bearer: String) -> Result<EntityId<DbApiKey>, ApiError> {
    let id = global.api_keys.verify_api_key(&bearer).await?;
    Ok(id)
}
