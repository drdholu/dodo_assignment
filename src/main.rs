use std::error::Error;

use axum::{Json, Router, extract::Path, http::StatusCode, response::IntoResponse, routing::get};
use dotenvy::dotenv;
use serde_json::{Value, json};
use sqlx::{Row, Connection};

#[derive(Debug)]
enum ApiError {
    NotFound, // 404
    InvalidInput(String), // 400
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
            ApiError::InvalidInput(msg) => (
                StatusCode::BAD_REQUEST, msg
            )
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

fn create_app() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/user/{id}", get(get_user))
}

async fn get_user(Path(id): Path<u32>) -> Result<Json<Value>, ApiError> {
    if id > 100 {return Err(ApiError::NotFound);}
    Ok(Json(json!({"id": id, "name": "Test"})))
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    // db conn
    println!("connecting to db");
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let pool = sqlx::postgres::PgPool::connect(&url)
        .await
        .expect("failed to connect to db");
    println!("db connected");
    
    let app = create_app();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind tcp lisenter");


    println!("server running");

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