use std::borrow::Cow;

use axum::{http::StatusCode, response::IntoResponse};
use axum_extra::extract::CookieJar;
use serde::Serialize;

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
