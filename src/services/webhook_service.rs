use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{db, models::transaction::Transaction};

#[derive(Debug)]
pub enum WebhookError {
    BadRequest(&'static str),
    Internal,
}

fn validate_url(url: &str) -> Result<(), WebhookError> {
    let u = url.trim();
    if u.is_empty() {
        return Err(WebhookError::BadRequest("url is required"));
    }
    if !(u.starts_with("http://") || u.starts_with("https://")) {
        return Err(WebhookError::BadRequest("url must start with http:// or https://"));
    }
    Ok(())
}

pub fn validate_create_endpoint(url: &str) -> Result<(), WebhookError> {
    validate_url(url)?;
    Ok(())
}

pub async fn enqueue_transaction_created_events_best_effort(
    pool: &PgPool,
    business_id: Uuid,
    tx: &Transaction,
) {
    let payload = json!({
        "event_type": "transaction.created",
        "timestamp": Utc::now(),
        "data": {
            "transaction_id": tx.id,
            "type": tx.tx_type,
            "source_account_id": tx.source_account_id,
            "dest_account_id": tx.dest_account_id,
            "amount": tx.amount,
            "created_at": tx.created_at,
        }
    });

    let payload_json = match serde_json::to_string(&payload) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("failed to serialize webhook payload");
            return;
        }
    };

    match db::enqueue_webhook_events_for_transaction(pool, business_id, tx.id, &payload_json).await {
        Ok(_count) => {}
        Err(err) => {
            eprintln!("failed to enqueue webhook events: {err}");
        }
    }
}


