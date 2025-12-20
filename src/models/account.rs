use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Account {
    pub id: Uuid,
    pub business_id: Uuid,
    pub name: String,
    pub currency: String,
    pub balance: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateAccountRequest {
    pub name: String,
    pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct AccountResponse {
    pub id: Uuid,
    pub name: String,
    pub currency: String,
    pub balance: i64,
}

impl From<Account> for AccountResponse {
    fn from(a: Account) -> Self {
        Self {
            id: a.id,
            name: a.name,
            currency: a.currency.trim().to_string(),
            balance: a.balance,
        }
    }
}


