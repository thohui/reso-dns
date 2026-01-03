use axum::{
    Json, Router,
    extract::{Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use axum_extra::extract::CookieJar;
use serde::Deserialize;

use crate::{
    database::models::{user::User, user_session::UserSession},
    global::SharedGlobal,
    utils::password,
};

use super::{cookie, error::ApiError};

pub mod middleware;

pub fn create_auth_router() -> Router<SharedGlobal> {
    Router::new().route("/signin", post(sign_in))
}

#[derive(Deserialize)]
pub(crate) struct SignInPayload {
    username: String,
    password: String,
}
pub async fn sign_in(
    global: State<SharedGlobal>,
    jar: CookieJar,
    payload: Json<SignInPayload>,
) -> axum::response::Result<Response, ApiError> {
    let user = match User::find_by_name(&global.database, payload.username.clone()).await {
        Ok(user) => user,
        Err(_) => {
            return Err(ApiError::invalid_credentials());
        }
    };

    // verify passwords,
    if let Err(_) = password::verify_password(&payload.password, &user.password_hash) {
        return Err(ApiError::invalid_credentials());
    }

    let session = UserSession::new(user.id);
    let session_id = session.id.clone();
    if let Err(e) = session.insert(&global.database).await {
        tracing::error!("failed to insert user session: {:?}", e);
        return Err(ApiError::authentication_required());
    }

    let encrypted_cookie = match cookie::encrypt_session_id(&global.cipher, session_id) {
        Ok(encrypted) => encrypted,
        Err(e) => {
            tracing::error!("failed to encrypt the session id: {:?}", e);
            return Err(ApiError::authentication_required());
        }
    };

    let cookie = cookie::build_session_cookie(encrypted_cookie);
    let jar = jar.add(cookie);
    Ok((jar, StatusCode::OK).into_response())
}
