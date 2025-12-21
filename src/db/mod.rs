pub mod pool;

use uuid::Uuid;

use sqlx::PgPool;
use chrono::{DateTime, Utc};

use crate::models::api_key::ApiKeyLookup;
use crate::models::account::Account;
use crate::models::transaction::{Transaction, TransactionType};
use crate::models::webhook::WebhookEndpoint;

pub async fn find_active_api_key_by_hash(
    pool: &PgPool,
    key_hash: &str,
) -> Result<Option<ApiKeyLookup>, sqlx::Error> {
    let q = "SELECT id, business_id FROM api_keys WHERE key_hash = $1 AND revoked_at IS NULL";

    let row: Option<(Uuid, Uuid)> = sqlx::query_as(q)
        .bind(key_hash)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|(id, business_id)| ApiKeyLookup { id, business_id }))
}

pub async fn create_account(
    pool: &PgPool,
    business_id: Uuid,
    name: &str,
    currency: &str,
) -> Result<Account, sqlx::Error> {
    let q = r#"
        INSERT INTO accounts (business_id, name, currency, balance)
        VALUES ($1, $2, $3, 0)
        RETURNING id, business_id, name, currency, balance
    "#;

    let row: (Uuid, Uuid, String, String, i64) = sqlx::query_as(q)
        .bind(business_id)
        .bind(name)
        .bind(currency)
        .fetch_one(pool)
        .await?;

    Ok(Account {
        id: row.0,
        business_id: row.1,
        name: row.2,
        currency: row.3,
        balance: row.4,
    })
}

pub async fn list_accounts(pool: &PgPool, business_id: Uuid) -> Result<Vec<Account>, sqlx::Error> {
    let q = r#"
        SELECT id, business_id, name, currency, balance
        FROM accounts
        WHERE business_id = $1
        ORDER BY created_at DESC
    "#;

    let rows: Vec<(Uuid, Uuid, String, String, i64)> = sqlx::query_as(q)
        .bind(business_id)
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|(id, business_id, name, currency, balance)| Account {
            id,
            business_id,
            name,
            currency,
            balance,
        })
        .collect())
}

pub async fn get_account(
    pool: &PgPool,
    business_id: Uuid,
    account_id: Uuid,
) -> Result<Option<Account>, sqlx::Error> {
    let q = r#"
        SELECT id, business_id, name, currency, balance
        FROM accounts
        WHERE id = $1 AND business_id = $2
        LIMIT 1
    "#;

    let row: Option<(Uuid, Uuid, String, String, i64)> = sqlx::query_as(q)
        .bind(account_id)
        .bind(business_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|(id, business_id, name, currency, balance)| Account {
        id,
        business_id,
        name,
        currency,
        balance,
    }))
}

pub async fn list_transactions(
    pool: &PgPool,
    business_id: Uuid,
) -> Result<Vec<Transaction>, sqlx::Error> {
    let q = r#"
        SELECT id, type::text, source_account_id, dest_account_id, amount, created_at
        FROM transactions
        WHERE business_id = $1
        ORDER BY created_at DESC
        LIMIT 100
    "#;

    let rows: Vec<(Uuid, String, Option<Uuid>, Option<Uuid>, i64, DateTime<Utc>)> = sqlx::query_as(q)
        .bind(business_id)
        .fetch_all(pool)
        .await?;

    let mut out = Vec::with_capacity(rows.len());
    for (id, tx_type, source_account_id, dest_account_id, amount, created_at) in rows {
        let tx_type = match tx_type.as_str() {
            "credit" => TransactionType::Credit,
            "debit" => TransactionType::Debit,
            "transfer" => TransactionType::Transfer,
            _ => continue,
        };

        out.push(Transaction {
            id,
            business_id,
            tx_type,
            source_account_id,
            dest_account_id,
            amount,
            created_at,
        });
    }

    Ok(out)
}

pub async fn get_transaction(
    pool: &PgPool,
    business_id: Uuid,
    id: Uuid,
) -> Result<Option<Transaction>, sqlx::Error> {
    let q = r#"
        SELECT id, type::text, source_account_id, dest_account_id, amount, created_at
        FROM transactions
        WHERE business_id = $1 AND id = $2
        LIMIT 1
    "#;

    let row: Option<(Uuid, String, Option<Uuid>, Option<Uuid>, i64, DateTime<Utc>)> = sqlx::query_as(q)
        .bind(business_id)
        .bind(id)
        .fetch_optional(pool)
        .await?;

    let Some((id, tx_type, source_account_id, dest_account_id, amount, created_at)) = row else {
        return Ok(None);
    };

    let tx_type = match tx_type.as_str() {
        "credit" => TransactionType::Credit,
        "debit" => TransactionType::Debit,
        "transfer" => TransactionType::Transfer,
        _ => return Ok(None),
    };

    Ok(Some(Transaction {
        id,
        business_id,
        tx_type,
        source_account_id,
        dest_account_id,
        amount,
        created_at,
    }))
}

pub async fn create_webhook_endpoint(
    pool: &PgPool,
    business_id: Uuid,
    url: &str,
) -> Result<WebhookEndpoint, sqlx::Error> {
    let q = r#"
        INSERT INTO webhook_endpoints (business_id, url, secret, active)
        VALUES ($1, $2, '', true)
        RETURNING id, business_id, url, active, created_at
    "#;

    let row: (Uuid, Uuid, String, bool, DateTime<Utc>) = sqlx::query_as(q)
        .bind(business_id)
        .bind(url)
        .fetch_one(pool)
        .await?;

    Ok(WebhookEndpoint {
        id: row.0,
        business_id: row.1,
        url: row.2,
        active: row.3,
        created_at: row.4,
    })
}

pub async fn list_webhook_endpoints(
    pool: &PgPool,
    business_id: Uuid,
) -> Result<Vec<WebhookEndpoint>, sqlx::Error> {
    let q = r#"
        SELECT id, business_id, url, active, created_at
        FROM webhook_endpoints
        WHERE business_id = $1
        ORDER BY created_at DESC
    "#;

    let rows: Vec<(Uuid, Uuid, String, bool, DateTime<Utc>)> = sqlx::query_as(q)
        .bind(business_id)
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|(id, business_id, url, active, created_at)| WebhookEndpoint {
            id,
            business_id,
            url,
            active,
            created_at,
        })
        .collect())
}

pub async fn deactivate_webhook_endpoint(
    pool: &PgPool,
    business_id: Uuid,
    endpoint_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let q = r#"
        UPDATE webhook_endpoints
        SET active = false
        WHERE id = $1 AND business_id = $2
    "#;

    let result = sqlx::query(q)
        .bind(endpoint_id)
        .bind(business_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn enqueue_webhook_events_for_transaction(
    pool: &PgPool,
    business_id: Uuid,
    transaction_id: Uuid,
    payload_json: &str,
) -> Result<u64, sqlx::Error> {
    let q = r#"
        INSERT INTO webhook_events (endpoint_id, transaction_id, payload)
        SELECT id, $2, $3::jsonb
        FROM webhook_endpoints
        WHERE business_id = $1 AND active = true
    "#;

    let result = sqlx::query(q)
        .bind(business_id)
        .bind(transaction_id)
        .bind(payload_json)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

#[derive(Debug, Clone)]
pub struct DueWebhookEvent {
    pub event_id: Uuid,
    pub url: String,
    pub payload_json: String,
    pub attempts: i32,
}

pub async fn fetch_due_webhook_events(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<DueWebhookEvent>, sqlx::Error> {
    let q = r#"
        SELECT
            e.id AS event_id,
            w.url,
            e.payload::text AS payload_json,
            e.attempts
        FROM webhook_events e
        JOIN webhook_endpoints w ON w.id = e.endpoint_id
        WHERE
            e.status = 'pending'
            AND (e.next_retry_at IS NULL OR e.next_retry_at <= now())
            AND w.active = true
        ORDER BY e.created_at ASC
        LIMIT $1
    "#;

    let rows: Vec<(Uuid, String, String, i32)> = sqlx::query_as(q)
        .bind(limit)
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|(event_id, url, payload_json, attempts)| DueWebhookEvent {
            event_id,
            url,
            payload_json,
            attempts,
        })
        .collect())
}

pub async fn mark_webhook_event_delivered(pool: &PgPool, event_id: Uuid) -> Result<(), sqlx::Error> {
    let q = r#"
        UPDATE webhook_events
        SET status = 'delivered', next_retry_at = NULL
        WHERE id = $1
    "#;
    sqlx::query(q).bind(event_id).execute(pool).await?;
    Ok(())
}

pub async fn mark_webhook_event_failed(
    pool: &PgPool,
    event_id: Uuid,
    attempts: i32,
    next_retry_at: Option<DateTime<Utc>>,
    terminal: bool,
) -> Result<(), sqlx::Error> {
    let status = if terminal { "failed" } else { "pending" };
    let q = r#"
        UPDATE webhook_events
        SET status = $2::webhook_status, attempts = $3, next_retry_at = $4
        WHERE id = $1
    "#;
    sqlx::query(q)
        .bind(event_id)
        .bind(status)
        .bind(attempts)
        .bind(next_retry_at)
        .execute(pool)
        .await?;
    Ok(())
}

