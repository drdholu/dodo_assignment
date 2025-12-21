use uuid::Uuid;
use chrono::{DateTime, Utc};

use sqlx::{PgPool, Postgres, Transaction as SqlxTransaction};

use crate::models::transaction::{CreateTransactionRequest, Transaction, TransactionType};

#[derive(Debug)]
pub enum TransactionError {
    BadRequest(&'static str),
    NotFound,
    InsufficientFunds,
    Internal,
}

fn order_uuids(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a.as_bytes() <= b.as_bytes() {
        (a, b)
    } else {
        (b, a)
    }
}

async fn lock_account_for_update(
    tx: &mut SqlxTransaction<'_, Postgres>,
    business_id: Uuid,
    account_id: Uuid,
) -> Result<Option<(Uuid, String, i64)>, sqlx::Error> {
    // Returns: (id, currency, balance)
    let q = r#"
        SELECT id, currency, balance
        FROM accounts
        WHERE id = $1 AND business_id = $2
        FOR UPDATE
    "#;

    let row: Option<(Uuid, String, i64)> = sqlx::query_as(q)
        .bind(account_id)
        .bind(business_id)
        .fetch_optional(&mut **tx)
        .await?;

    Ok(row)
}

async fn update_balance(
    tx: &mut SqlxTransaction<'_, Postgres>,
    account_id: Uuid,
    new_balance: i64,
) -> Result<(), sqlx::Error> {
    let q = r#"
        UPDATE accounts
        SET balance = $1
        WHERE id = $2
    "#;

    sqlx::query(q)
        .bind(new_balance)
        .bind(account_id)
        .execute(&mut **tx)
        .await?;

    Ok(())
}

async fn insert_transaction_row(
    tx: &mut SqlxTransaction<'_, Postgres>,
    business_id: Uuid,
    tx_type: TransactionType,
    source_account_id: Option<Uuid>,
    dest_account_id: Option<Uuid>,
    amount: i64,
) -> Result<(Uuid, DateTime<Utc>), sqlx::Error> {
    let q = r#"
        INSERT INTO transactions (business_id, type, source_account_id, dest_account_id, amount)
        VALUES ($1, $2::transaction_type, $3, $4, $5)
        RETURNING id, created_at
    "#;

    let (id, created_at): (Uuid, DateTime<Utc>) = sqlx::query_as(q)
        .bind(business_id)
        .bind(tx_type.as_db_str())
        .bind(source_account_id)
        .bind(dest_account_id)
        .bind(amount)
        .fetch_one(&mut **tx)
        .await?;

    Ok((id, created_at))
}

pub async fn create_transaction(
    pool: &PgPool,
    business_id: Uuid,
    req: CreateTransactionRequest,
) -> Result<Transaction, TransactionError> {
    if req.amount <= 0 {
        return Err(TransactionError::BadRequest("amount must be > 0"));
    }

    let mut tx = pool.begin().await.map_err(|_| TransactionError::Internal)?;

    let tx_type = req.tx_type;
    let amount = req.amount;

    let (source_account_id, dest_account_id) = match tx_type {
        TransactionType::Credit => {
            if req.dest_account_id.is_none() || req.source_account_id.is_some() {
                return Err(TransactionError::BadRequest(
                    "credit requires dest_account_id and no source_account_id",
                ));
            }
            (None, req.dest_account_id)
        }
        TransactionType::Debit => {
            if req.source_account_id.is_none() || req.dest_account_id.is_some() {
                return Err(TransactionError::BadRequest(
                    "debit requires source_account_id and no dest_account_id",
                ));
            }
            (req.source_account_id, None)
        }
        TransactionType::Transfer => {
            if req.source_account_id.is_none() || req.dest_account_id.is_none() {
                return Err(TransactionError::BadRequest(
                    "transfer requires source_account_id and dest_account_id",
                ));
            }
            let s = req.source_account_id.unwrap();
            let d = req.dest_account_id.unwrap();
            if s == d {
                return Err(TransactionError::BadRequest(
                    "transfer requires distinct source and destination accounts",
                ));
            }
            (Some(s), Some(d))
        }
    };

    match tx_type {
        TransactionType::Credit => {
            let dest_id = dest_account_id.unwrap();

            // 1) Lock row
            let dest = lock_account_for_update(&mut tx, business_id, dest_id)
                .await
                .map_err(|_| TransactionError::Internal)?;

            let (_, _currency, dest_balance) = dest.ok_or(TransactionError::NotFound)?;

            // 2) Balance update + insert in same DB transaction
            let new_balance = dest_balance
                .checked_add(amount)
                .ok_or(TransactionError::Internal)?;

            update_balance(&mut tx, dest_id, new_balance)
                .await
                .map_err(|_| TransactionError::Internal)?;

            let (id, created_at) = insert_transaction_row(
                &mut tx,
                business_id,
                tx_type,
                None,
                Some(dest_id),
                amount,
            )
            .await
            .map_err(|_| TransactionError::Internal)?;

            tx.commit().await.map_err(|_| TransactionError::Internal)?;

            Ok(Transaction {
                id,
                business_id,
                tx_type,
                source_account_id: None,
                dest_account_id: Some(dest_id),
                amount,
                created_at,
            })
        }
        TransactionType::Debit => {
            let source_id = source_account_id.unwrap();

            // 1) Lock row
            let source = lock_account_for_update(&mut tx, business_id, source_id)
                .await
                .map_err(|_| TransactionError::Internal)?;

            let (_, _currency, source_balance) = source.ok_or(TransactionError::NotFound)?;

            // 2) Balance check (after lock)
            if source_balance < amount {
                return Err(TransactionError::InsufficientFunds);
            }

            // 3) Balance update + insert in same DB transaction
            let new_balance = source_balance
                .checked_sub(amount)
                .ok_or(TransactionError::Internal)?;

            update_balance(&mut tx, source_id, new_balance)
                .await
                .map_err(|_| TransactionError::Internal)?;

            let (id, created_at) = insert_transaction_row(
                &mut tx,
                business_id,
                tx_type,
                Some(source_id),
                None,
                amount,
            )
            .await
            .map_err(|_| TransactionError::Internal)?;

            tx.commit().await.map_err(|_| TransactionError::Internal)?;

            Ok(Transaction {
                id,
                business_id,
                tx_type,
                source_account_id: Some(source_id),
                dest_account_id: None,
                amount,
                created_at,
            })
        }
        TransactionType::Transfer => {
            let source_id = source_account_id.unwrap();
            let dest_id = dest_account_id.unwrap();

            // Deterministic lock order to avoid deadlocks.
            let (first, second) = order_uuids(source_id, dest_id);

            let first_row = lock_account_for_update(&mut tx, business_id, first)
                .await
                .map_err(|_| TransactionError::Internal)?;
            let second_row = lock_account_for_update(&mut tx, business_id, second)
                .await
                .map_err(|_| TransactionError::Internal)?;

            let (first_id, first_currency, first_balance) =
                first_row.ok_or(TransactionError::NotFound)?;
            let second_row = second_row.ok_or(TransactionError::NotFound)?;
            let second_currency = second_row.1;
            let second_balance = second_row.2;

            // Re-associate balances to source/dest
            let (source_currency, source_balance, dest_currency, dest_balance) = if first_id == source_id {
                (first_currency, first_balance, second_currency, second_balance)
            } else {
                (second_currency, second_balance, first_currency, first_balance)
            };

            if source_currency.trim() != dest_currency.trim() {
                return Err(TransactionError::BadRequest(
                    "transfer requires source and destination accounts to have same currency",
                ));
            }

            // Balance check (after locks)
            if source_balance < amount {
                return Err(TransactionError::InsufficientFunds);
            }

            let new_source_balance = source_balance
                .checked_sub(amount)
                .ok_or(TransactionError::Internal)?;
            let new_dest_balance = dest_balance
                .checked_add(amount)
                .ok_or(TransactionError::Internal)?;

            update_balance(&mut tx, source_id, new_source_balance)
                .await
                .map_err(|_| TransactionError::Internal)?;
            update_balance(&mut tx, dest_id, new_dest_balance)
                .await
                .map_err(|_| TransactionError::Internal)?;

            let (id, created_at) = insert_transaction_row(
                &mut tx,
                business_id,
                tx_type,
                Some(source_id),
                Some(dest_id),
                amount,
            )
            .await
            .map_err(|_| TransactionError::Internal)?;

            tx.commit().await.map_err(|_| TransactionError::Internal)?;

            Ok(Transaction {
                id,
                business_id,
                tx_type,
                source_account_id: Some(source_id),
                dest_account_id: Some(dest_id),
                amount,
                created_at,
            })
        }
    }
}


