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
    models::account::{AccountResponse, CreateAccountRequest},
    state::AppState,
};

fn normalize_currency(input: &str) -> Option<String> {
    let c = input.trim().to_ascii_uppercase();
    let ok = c.len() == 3 && c.chars().all(|ch| ch.is_ascii_alphabetic());
    ok.then_some(c)
}

fn normalize_name(input: &str) -> Option<String> {
    let name = input.trim();
    if name.is_empty() {
        return None;
    }
    if name.len() > 128 {
        return None;
    }
    Some(name.to_string())
}

fn is_unique_violation(err: &sqlx::Error) -> bool {
    let sqlx::Error::Database(db_err) = err else {
        return false;
    };

    db_err.code().is_some_and(|c| c == "23505")
}

pub async fn create_account(
    State(state): State<AppState>,
    Extension(ctx): Extension<BusinessContext>,
    Json(payload): Json<CreateAccountRequest>,
) -> impl IntoResponse {
    let name = match normalize_name(&payload.name) {
        Some(n) => n,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "name is required (1-128 chars)" })),
            )
                .into_response();
        }
    };

    let currency = match normalize_currency(&payload.currency) {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "currency must be a 3-letter code, e.g. USD" })),
            )
                .into_response();
        }
    };

    match db::create_account(&state.pool, ctx.business_id, &name, &currency).await {
        Ok(account) => (StatusCode::CREATED, Json(AccountResponse::from(account))).into_response(),
        Err(e) if is_unique_violation(&e) => (
            StatusCode::CONFLICT,
            Json(json!({ "error": "account name already exists for this business" })),
        )
            .into_response(),
        Err(_) => ApiError::InternalError.into_response(),
    }
}

pub async fn list_accounts(
    State(state): State<AppState>,
    Extension(ctx): Extension<BusinessContext>,
) -> impl IntoResponse {
    match db::list_accounts(&state.pool, ctx.business_id).await {
        Ok(accounts) => {
            let resp: Vec<AccountResponse> =
                accounts.into_iter().map(AccountResponse::from).collect();
            Json(resp).into_response()
        }
        Err(_) => ApiError::InternalError.into_response(),
    }
}

pub async fn get_account(
    State(state): State<AppState>,
    Extension(ctx): Extension<BusinessContext>,
    Path(account_id): Path<Uuid>,
) -> impl IntoResponse {
    match db::get_account(&state.pool, ctx.business_id, account_id).await {
        Ok(Some(account)) => Json(AccountResponse::from(account)).into_response(),
        Ok(None) => ApiError::NotFound.into_response(),
        Err(_) => ApiError::InternalError.into_response(),
    }
}
