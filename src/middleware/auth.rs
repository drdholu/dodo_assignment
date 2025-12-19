use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::{error::ApiError, state::AppState};

#[derive(Clone, Debug)]
pub struct BusinessContext {
    pub business_id: uuid::Uuid,
    pub api_key_id: uuid::Uuid,
}

pub async fn api_key_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {

    let raw_api_key = req
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or(ApiError::Unauthorized)?;

    let key_hash = hmac_sha256_hex(&state.hmac_secret, raw_api_key);

    let q = "SELECT id, business_id FROM api_keys WHERE key_hash = $1 AND revoked_at IS NULL";
    let row: Option<(uuid::Uuid, uuid::Uuid)> = sqlx::query_as(q)
        .bind(&key_hash)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| ApiError::InternalError)?;

    let (api_key_id, business_id) = row.ok_or(ApiError::Unauthorized)?;

    req.extensions_mut().insert(BusinessContext {
        business_id,
        api_key_id,
    });

    Ok(next.run(req).await)
}

fn hmac_sha256_hex(secret: &str, msg: &str) -> String {
    type HmacSha256 = Hmac<Sha256>;

    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key size");
    mac.update(msg.as_bytes());

    hex::encode(mac.finalize().into_bytes())
}