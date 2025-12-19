use axum::{
    Json,
    extract::State,
    response::IntoResponse,
};
use serde_json::json;

use crate::{error::ApiError, state::AppState};

pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "OK",
        "message": "Server is running"
    }))
}

pub async fn db_health_check(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    sqlx::query("SELECT 1")
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::InternalError)?;

    Ok(Json(json!({
        "status": "OK",
        "message": "DB reachable"
    })))
}

