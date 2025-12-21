use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;


pub async fn create_pool(database_url: &str) -> PgPool {
    // Keep startup resilient in docker-compose: Postgres may take a few seconds to accept connections.
    // This is intentionally simple (single-process setup), but prevents "crash loop on boot".
    let mut last_err: Option<sqlx::Error> = None;

    for attempt in 1..=30 {
        match PgPoolOptions::new()
            .max_connections(10)
            .min_connections(1)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await
        {
            Ok(pool) => return pool,
            Err(err) => {
                last_err = Some(err);
                eprintln!("db connect attempt {attempt}/30 failed; retrying...");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    panic!(
        "failed to create Postgres connection pool after retries: {:?}",
        last_err
    );
}