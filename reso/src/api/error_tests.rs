#[cfg(test)]
mod tests {
    use super::super::error::ApiError;
    use axum::http::StatusCode;

    #[test]
    fn test_invalid_credentials_error() {
        let error = ApiError::invalid_credentials();
        assert_eq!(error.status_code, StatusCode::UNAUTHORIZED);
        assert_eq!(error.error.as_ref(), "unauthorized");
        assert_eq!(error.message.as_ref(), "Invalid credentials.");
    }

    #[test]
    fn test_authentication_required_error() {
        let error = ApiError::authentication_required();
        assert_eq!(error.status_code, StatusCode::UNAUTHORIZED);
        assert_eq!(error.error.as_ref(), "authentication_required");
        assert_eq!(error.message.as_ref(), "Authentication required.");
    }

    #[test]
    fn test_session_expired_error() {
        let error = ApiError::session_expired();
        assert_eq!(error.status_code, StatusCode::UNAUTHORIZED);
        assert_eq!(error.error.as_ref(), "session_expired");
        assert_eq!(error.message.as_ref(), "The session has expired.");
    }

    #[test]
    fn test_server_error() {
        let error = ApiError::server_error();
        assert_eq!(error.status_code, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(error.error.as_ref(), "server_error");
        assert_eq!(error.message.as_ref(), "Something went wrong.");
    }

    #[test]
    fn test_error_serialization() {
        let error = ApiError::invalid_credentials();
        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(json["error"], "unauthorized");
        assert_eq!(json["message"], "Invalid credentials.");
        assert!(json.get("status_code").is_none());
        assert!(json.get("jar").is_none());
    }

    #[test]
    fn test_default_error() {
        let error = ApiError::default();
        assert_eq!(error.status_code, StatusCode::OK);
        assert_eq!(error.error.as_ref(), "");
        assert_eq!(error.message.as_ref(), "");
    }

    #[test]
    fn test_cookie_jar_builder() {
        use axum_extra::extract::CookieJar;

        let jar = CookieJar::new();
        let error = ApiError::server_error().cookie_jar(jar.clone());
        assert_eq!(error.status_code, StatusCode::INTERNAL_SERVER_ERROR);
    }
}