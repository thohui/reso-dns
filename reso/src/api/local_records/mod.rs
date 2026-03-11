use crate::{database::models::local_record::LocalRecord, global::SharedGlobal};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    middleware,
    routing::{delete, get, patch, post},
};
use serde::Deserialize;

use super::{
    auth::middleware::auth_middleware,
    error::ApiError,
    pagination::{PagedQuery, PagedResponse},
};

pub fn create_local_records_router(global: SharedGlobal) -> Router<SharedGlobal> {
    Router::new()
        .route("/", get(list))
        .route("/", post(add_record))
        .route("/", delete(remove_record))
        .route("/toggle", patch(toggle_record))
        .layer(middleware::from_fn_with_state(global, auth_middleware))
}

pub async fn list(
    query: Query<PagedQuery>,
    global: State<SharedGlobal>,
) -> Result<Json<PagedResponse<LocalRecord>>, ApiError> {
    let top = query.top();
    let skip = query.skip();

    let db_top: i64 = top.try_into().map_err(|_| ApiError::bad_request())?;
    let db_skip: i64 = skip.try_into().map_err(|_| ApiError::bad_request())?;

    let records = LocalRecord::list(&global.core_database, db_top, db_skip)
        .await
        .map_err(|e| {
            tracing::error!("failed to list local records: {:?}", e);
            ApiError::server_error()
        })?;

    let count: u64 = LocalRecord::row_count(&global.core_database)
        .await
        .map_err(|e| {
            tracing::error!("failed to get local record count: {:?}", e);
            ApiError::server_error()
        })?
        .try_into()
        .map_err(|_| ApiError::server_error())?;

    Ok(Json(PagedResponse::new(records, Some(count), top, skip)))
}

#[derive(Deserialize)]
pub struct AddRecordPayload {
    name: String,
    record_type: u16,
    value: String,
    #[serde(default = "default_ttl")]
    ttl: u32,
}

fn default_ttl() -> u32 {
    300
}

pub async fn add_record(
    global: State<SharedGlobal>,
    Json(payload): Json<AddRecordPayload>,
) -> Result<StatusCode, ApiError> {
    global
        .local_records_service
        .add_record(&payload.name, payload.record_type, &payload.value, payload.ttl)
        .await
        .map_err(|e| {
            tracing::error!("failed to add local record: {:?}", e);
            ApiError::server_error()
        })?;
    Ok(StatusCode::CREATED)
}

#[derive(Deserialize)]
pub struct IdPayload {
    id: i64,
}

pub async fn remove_record(
    global: State<SharedGlobal>,
    Json(payload): Json<IdPayload>,
) -> Result<(), ApiError> {
    global.local_records_service.remove_record(payload.id).await.map_err(|e| {
        tracing::error!("failed to remove local record: {:?}", e);
        ApiError::server_error()
    })?;
    Ok(())
}

pub async fn toggle_record(
    global: State<SharedGlobal>,
    Json(payload): Json<IdPayload>,
) -> Result<(), ApiError> {
    global.local_records_service.toggle_record(payload.id).await.map_err(|e| {
        tracing::error!("failed to toggle local record: {:?}", e);
        ApiError::server_error()
    })?;
    Ok(())
}
