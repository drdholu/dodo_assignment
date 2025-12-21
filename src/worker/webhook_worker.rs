use chrono::{DateTime, Utc};
use reqwest::Client;
use sqlx::PgPool;
use std::time::Duration;

use crate::db;

const POLL_INTERVAL: Duration = Duration::from_secs(2);
const BATCH_SIZE: i64 = 25;
const MAX_ATTEMPTS: i32 = 5;

fn compute_next_retry(attempts: i32) -> DateTime<Utc> {
    // Exponential backoff capped at 5 minutes.
    // attempts is the *new* attempts count (after increment).
    let exp = 2_i64.saturating_pow(attempts.min(10) as u32);
    let seconds = (5_i64.saturating_mul(exp)).min(300);
    Utc::now() + chrono::Duration::seconds(seconds)
}

async fn deliver_one(client: &Client, url: &str, payload_json: &str) -> Result<(), reqwest::Error> {
    client
        .post(url)
        .header("content-type", "application/json")
        .body(payload_json.to_string())
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

pub async fn run(pool: PgPool) {
    let client = Client::new();

    loop {
        let due = match db::fetch_due_webhook_events(&pool, BATCH_SIZE).await {
            Ok(rows) => rows,
            Err(err) => {
                eprintln!("webhook worker: failed to fetch due events: {err}");
                tokio::time::sleep(POLL_INTERVAL).await;
                continue;
            }
        };

        if due.is_empty() {
            tokio::time::sleep(POLL_INTERVAL).await;
            continue;
        }

        for ev in due {
            let result = deliver_one(&client, &ev.url, &ev.payload_json).await;
            match result {
                Ok(()) => {
                    if let Err(err) = db::mark_webhook_event_delivered(&pool, ev.event_id).await {
                        eprintln!("webhook worker: failed to mark delivered {}: {err}", ev.event_id);
                    }
                }
                Err(err) => {
                    let new_attempts = ev.attempts.saturating_add(1);
                    let terminal = new_attempts >= MAX_ATTEMPTS;
                    let next_retry_at = if terminal {
                        None
                    } else {
                        Some(compute_next_retry(new_attempts))
                    };

                    if let Err(db_err) = db::mark_webhook_event_failed(
                        &pool,
                        ev.event_id,
                        new_attempts,
                        next_retry_at,
                        terminal,
                    )
                    .await
                    {
                        eprintln!(
                            "webhook worker: failed to mark failed {}: {db_err}",
                            ev.event_id
                        );
                    }

                    eprintln!(
                        "webhook worker: delivery failed event={} attempts={} terminal={} err={err}",
                        ev.event_id, new_attempts, terminal
                    );
                }
            }
        }
    }
}


