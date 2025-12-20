use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::{
    db,
    error::ApiError,
    models::api_key::ApiKeyLookup,
    state::AppState,
};

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

    // println!("here");
    let raw_api_key = req
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or(ApiError::Unauthorized)?;

    let key_hash = hmac_sha256_hex(&state.hmac_secret, raw_api_key);

    let api_key: ApiKeyLookup = db::find_active_api_key_by_hash(&state.pool, &key_hash)
        .await
        .map_err(|_| ApiError::InternalError)?
        .ok_or(ApiError::Unauthorized)?;

    req.extensions_mut().insert(BusinessContext {
        business_id: api_key.business_id,
        api_key_id: api_key.id,
    });

    // println!("here 2");
    Ok(next.run(req).await)
}

fn hmac_sha256_hex(secret: &str, msg: &str) -> String {
    type HmacSha256 = Hmac<Sha256>;

    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key size");
    mac.update(msg.as_bytes());

    hex::encode(mac.finalize().into_bytes())
}