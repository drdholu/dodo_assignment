pub mod pool;

use uuid::Uuid;

use sqlx::PgPool;

use crate::models::api_key::ApiKeyLookup;
use crate::models::account::Account;

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

