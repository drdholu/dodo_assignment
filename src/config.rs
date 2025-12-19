use std::env;

use dotenvy::dotenv;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub server_port: u16,
    pub hmac_secret: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv().ok();

        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");

        let server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .expect("SERVER_PORT must be a valid u16");

        let hmac_secret = env::var("HMAC_SECRET")
            .expect("HMAC_SECRET must be set");

        Self {
            database_url,
            server_port,
            hmac_secret,
        }
    }
}
