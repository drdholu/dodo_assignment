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
    models::transaction::{CreateTransactionRequest, TransactionResponse},
    services::transaction_service::{TransactionError, create_transaction},
    state::AppState,
};

pub async fn create_transaction_handler(
    State(state): State<AppState>,
    Extension(ctx): Extension<BusinessContext>,
    Json(payload): Json<CreateTransactionRequest>,
) -> impl IntoResponse {
    let tx = match create_transaction(&state.pool, ctx.business_id, payload).await {
        Ok(t) => t,
        Err(TransactionError::BadRequest(msg)) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response();
        }
        Err(TransactionError::NotFound) => {
            return ApiError::NotFound.into_response();
        }
        Err(TransactionError::InsufficientFunds) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "insufficient funds" })),
            )
                .into_response();
        }
        Err(TransactionError::Internal) => {
            return ApiError::InternalError.into_response();
        }
    };

    (StatusCode::CREATED, Json(TransactionResponse::from(tx))).into_response()
}

pub async fn list_transactions(
    State(state): State<AppState>,
    Extension(ctx): Extension<BusinessContext>,
) -> impl IntoResponse {
    match db::list_transactions(&state.pool, ctx.business_id).await {
        Ok(rows) => {
            let resp: Vec<TransactionResponse> =
                rows.into_iter().map(TransactionResponse::from).collect();
            Json(resp).into_response()
        }
        Err(_) => ApiError::InternalError.into_response(),
    }
}

pub async fn get_transaction(
    State(state): State<AppState>,
    Extension(ctx): Extension<BusinessContext>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match db::get_transaction(&state.pool, ctx.business_id, id).await {
        Ok(Some(row)) => Json(TransactionResponse::from(row)).into_response(),
        Ok(None) => ApiError::NotFound.into_response(),
        Err(_) => ApiError::InternalError.into_response(),
    }
}


