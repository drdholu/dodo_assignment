use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;


pub async fn create_pool(database_url: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(10)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
        .expect("failed to create Postgres connection pool")
}