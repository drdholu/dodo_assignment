use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Credit,
    Debit,
    Transfer,
}

// convert rust enum for psql
impl TransactionType {
    pub fn as_db_str(self) -> &'static str {
        match self {
            TransactionType::Credit => "credit",
            TransactionType::Debit => "debit",
            TransactionType::Transfer => "transfer",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: Uuid,
    pub business_id: Uuid,
    pub tx_type: TransactionType,
    pub source_account_id: Option<Uuid>,
    pub dest_account_id: Option<Uuid>,
    pub amount: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTransactionRequest {
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    pub amount: i64,
    pub source_account_id: Option<Uuid>,
    pub dest_account_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct TransactionResponse {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    pub source_account_id: Option<Uuid>,
    pub dest_account_id: Option<Uuid>,
    pub amount: i64,
    pub created_at: DateTime<Utc>,
}

impl From<Transaction> for TransactionResponse {
    fn from(t: Transaction) -> Self {
        Self {
            id: t.id,
            tx_type: t.tx_type,
            source_account_id: t.source_account_id,
            dest_account_id: t.dest_account_id,
            amount: t.amount,
            created_at: t.created_at,
        }
    }
}


