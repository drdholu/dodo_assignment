use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ApiKeyLookup {
    pub id: Uuid,
    pub business_id: Uuid,
}


