use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    db,
    error::ApiError,
    middleware::auth::BusinessContext,
    models::webhook::{CreateWebhookEndpointRequest, WebhookEndpointResponse},
    services::webhook_service::{WebhookError, validate_create_endpoint},
    state::AppState,
};

pub async fn create_webhook_endpoint(
    State(state): State<AppState>,
    Extension(ctx): Extension<BusinessContext>,
    Json(payload): Json<CreateWebhookEndpointRequest>,
) -> impl IntoResponse {
    if let Err(e) = validate_create_endpoint(&payload.url) {
        return match e {
            WebhookError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response()
            }
            WebhookError::Internal => ApiError::InternalError.into_response(),
        };
    }

    match db::create_webhook_endpoint(&state.pool, ctx.business_id, &payload.url).await {
        Ok(row) => (
            StatusCode::CREATED,
            Json(WebhookEndpointResponse::from(row)),
        )
            .into_response(),
        Err(_) => ApiError::InternalError.into_response(),
    }
}

pub async fn list_webhook_endpoints(
    State(state): State<AppState>,
    Extension(ctx): Extension<BusinessContext>,
) -> impl IntoResponse {
    match db::list_webhook_endpoints(&state.pool, ctx.business_id).await {
        Ok(rows) => {
            let resp: Vec<WebhookEndpointResponse> =
                rows.into_iter().map(WebhookEndpointResponse::from).collect();
            Json(resp).into_response()
        }
        Err(_) => ApiError::InternalError.into_response(),
    }
}

pub async fn delete_webhook_endpoint(
    State(state): State<AppState>,
    Extension(ctx): Extension<BusinessContext>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match db::deactivate_webhook_endpoint(&state.pool, ctx.business_id, id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => ApiError::NotFound.into_response(),
        Err(_) => ApiError::InternalError.into_response(),
    }
}


