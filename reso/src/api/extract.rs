use std::ops::Deref;

use axum::{
    Json,
    extract::{FromRequest, FromRequestParts, Path, Query, Request},
    http::request::Parts,
};
use serde::{Deserialize, Deserializer, de::DeserializeOwned};

use super::error::ApiError;

/// Wrapper around axum's Query that returns an ApiError on Rejection
pub struct ApiQuery<T>(pub T);

impl<T, S> FromRequestParts<S> for ApiQuery<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state).await?;
        Ok(Self(value))
    }
}

impl<T> Deref for ApiQuery<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

/// Wrapper around axum's Json that returns an ApiError on Rejection
pub struct ApiJson<T>(pub T);

impl<T, S> FromRequest<S> for ApiJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await?;
        Ok(Self(value))
    }
}

impl<T> Deref for ApiJson<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

/// Wrapper around axum's Path that returns an ApiError on Rejection
pub struct ApiPath<T>(pub T);

impl<T, S> FromRequestParts<S> for ApiPath<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(value) = Path::<T>::from_request_parts(parts, state).await?;
        Ok(Self(value))
    }
}

impl<T> Deref for ApiPath<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

/// serde helper that treats `?field=` as `None` instead of failing to parse.
pub fn empty_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match Option::<String>::deserialize(de)?.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(s) => s.parse().map(Some).map_err(serde::de::Error::custom),
    }
}
