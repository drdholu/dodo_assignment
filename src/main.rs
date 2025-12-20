use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{get, post},
};

use dodo_assign::{
    config::Config,
    db::pool::create_pool,
    handlers::health::{db_health_check, health_check},
    middleware::auth::api_key_auth,
    state::AppState,
    handlers::accounts,
};

fn create_app(state: AppState) -> Router {
    let protected = Router::new()
        .route("/create-account", post(accounts::create_account))
        .route("/accounts", get(accounts::list_accounts))
        .route("/accounts/{id}", get(accounts::get_account))
        .layer(from_fn_with_state(state.clone(), api_key_auth));

    Router::new()
        .route("/health", get(health_check))
        .route("/health/db", get(db_health_check))
        .nest("/api", protected)
        .with_state(state.clone())
}

#[tokio::main]
async fn main() {
    let config = Config::from_env();

    println!("connecting to db");
    let pool = create_pool(&config.database_url).await;
    println!("db connected");

    let state = AppState {
        pool,
        hmac_secret: config.hmac_secret,
    };

    let app = create_app(state);

    let bind_addr = format!("0.0.0.0:{}", config.server_port);
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("failed to bind tcp lisenter");

    println!("server running on {bind_addr}");

    axum::serve(listener, app)
        .await
        .expect("failed to start server");
}