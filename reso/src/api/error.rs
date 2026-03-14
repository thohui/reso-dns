use std::borrow::Cow;

use axum::{http::StatusCode, response::IntoResponse};
use axum_extra::extract::CookieJar;
use serde::Serialize;

use crate::services::ServiceError;

#[derive(Debug, Clone, Default, Serialize)]
pub struct ApiError {
    #[serde(skip)]
    pub status_code: StatusCode,
    pub error: Cow<'static, str>,
    pub message: Cow<'static, str>,
    #[serde(skip_serializing)]
    jar: Option<CookieJar>,
}

impl ApiError {
    pub fn invalid_credentials() -> Self {
        Self {
            status_code: StatusCode::UNAUTHORIZED,
            error: Cow::Borrowed("unauthorized"),
            message: Cow::Borrowed("Invalid credentials."),
            jar: None,
        }
    }
    pub fn authentication_required() -> Self {
        Self {
            status_code: StatusCode::UNAUTHORIZED,
            error: Cow::Borrowed("authentication_required"),
            message: Cow::Borrowed("Authentication required."),
            jar: None,
        }
    }
    pub fn session_expired() -> Self {
        Self {
            status_code: StatusCode::UNAUTHORIZED,
            error: Cow::Borrowed("session_expired"),
            message: Cow::Borrowed("The session has expired."),
            jar: None,
        }
    }
    pub fn setup_already_completed() -> Self {
        Self {
            status_code: StatusCode::CONFLICT,
            error: Cow::Borrowed("setup_already_completed"),
            message: Cow::Borrowed("Setup has already been completed."),
            jar: None,
        }
    }
    pub fn bad_request() -> Self {
        Self {
            status_code: StatusCode::BAD_REQUEST,
            error: Cow::Borrowed("bad_request"),
            message: Cow::Borrowed("Bad request."),
            jar: None,
        }
    }
    pub fn server_error() -> Self {
        Self {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            error: Cow::Borrowed("server_error"),
            message: Cow::Borrowed("Something went wrong."),
            jar: None,
        }
    }

    pub fn cookie_jar(self, jar: CookieJar) -> Self {
        Self { jar: Some(jar), ..self }
    }
}

impl From<ServiceError> for ApiError {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::BadRequest(msg) => Self {
                status_code: StatusCode::BAD_REQUEST,
                error: Cow::Borrowed("bad_request"),
                message: Cow::Owned(msg),
                jar: None,
            },
            ServiceError::Conflict(msg) => Self {
                status_code: StatusCode::CONFLICT,
                error: Cow::Borrowed("conflict"),
                message: Cow::Owned(msg),
                jar: None,
            },
            ServiceError::NotFound(msg) => Self {
                status_code: StatusCode::NOT_FOUND,
                error: Cow::Borrowed("not_found"),
                message: Cow::Owned(msg),
                jar: None,
            },
            ServiceError::Internal(err) => {
                tracing::error!("internal error: {:?}", err);
                Self::server_error()
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = self.status_code;
        let jar = self.jar.clone();

        let resp = (status, axum::Json(self));

        match jar {
            Some(jar) => (jar, resp).into_response(),
            None => resp.into_response(),
        }
    }
}
