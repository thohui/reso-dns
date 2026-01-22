use std::borrow::Cow;

use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
pub struct ApiError {
    #[serde(skip)]
    pub status_code: StatusCode,
    pub error: Cow<'static, str>,
    pub message: Cow<'static, str>,
}

impl ApiError {
    pub fn invalid_credentials() -> Self {
        Self {
            status_code: StatusCode::UNAUTHORIZED,
            error: Cow::Borrowed("unauthorized"),
            message: Cow::Borrowed("Invalid credentials."),
        }
    }
    pub fn authentication_required() -> Self {
        Self {
            status_code: StatusCode::UNAUTHORIZED,
            error: Cow::Borrowed("authentication_required"),
            message: Cow::Borrowed("Authentication required."),
        }
    }
    pub fn session_expired() -> Self {
        Self {
            status_code: StatusCode::UNAUTHORIZED,
            error: Cow::Borrowed("session_expired"),
            message: Cow::Borrowed("The session has expired."),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (self.status_code, Json(self)).into_response()
    }
}
