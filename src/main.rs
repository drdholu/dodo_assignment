use axum::{
    Json,
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde_json::{Value, json};
use sqlx::PgPool;

use dodo_assign::{config::Config, db::pool::create_pool};

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

#[derive(Debug)]
enum ApiError {
    NotFound, // 404
    // InvalidInput(String), // 400
    InternalError // 500
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            ApiError::NotFound => (
                StatusCode::NOT_FOUND, "Data Not found".to_string()
            ),
            ApiError::InternalError => (
                StatusCode::INTERNAL_SERVER_ERROR, "Internal Sevrer error".to_string()
            ),
            // ApiError::InvalidInput(msg) => (
            //     StatusCode::BAD_REQUEST, msg
            // )
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

async fn health_check() -> impl IntoResponse {

    //TODO: make sure to standardize API responses for all API's.
    Json(json!({
        "status": "OK",
        "message": "Server is running"
    }))
}

async fn db_health_check(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    sqlx::query("SELECT 1")
        .execute(&state.pool)
        .await
        .map_err(|_| ApiError::InternalError)?;

    Ok(Json(json!({
        "status": "OK",
        "message": "DB reachable"
    })))
}

fn create_app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/health/db", get(db_health_check))
        .route("/user/{id}", get(get_user))
        .with_state(state)
}

async fn get_user(Path(id): Path<u32>) -> Result<Json<Value>, ApiError> {
    if id > 100 {return Err(ApiError::NotFound);}
    Ok(Json(json!({"id": id, "name": "Test"})))
}

#[tokio::main]
async fn main() {
    let config = Config::from_env();

    println!("connecting to db");
    let pool = create_pool(&config.database_url).await;
    println!("db connected");

    let state = AppState { pool };
    let app = create_app(state);

    let bind_addr = format!("0.0.0.0:{}", config.server_port);
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("failed to bind tcp lisenter");


    println!("server running on {bind_addr}");

    // connect listener to app
    axum::serve(listener, app)
        .await
        .expect("failed to start server");
}

// #[cfg(test)]
// mod tests {

//     use super::*;

//     #[tokio::test]


// }