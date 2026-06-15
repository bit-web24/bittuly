use std::env;

pub struct AuthConfig {
    pub database_url: String,
    pub server_port: u16,
    pub mode: String,
    pub server_addr: String,
    pub cors_origin: String,
}

pub struct UrlConfig {
    pub database_url: String,
    pub redis_url: String,
    pub server_port: u16,
    pub server_addr: String,
    pub mode: String,
    pub cors_origin: String,
}

impl AuthConfig {
    pub fn from_env() -> Result<Self, env::VarError> {
        dotenvy::dotenv().ok();

        Ok(Self {
            database_url: env::var("AUTH_DATABASE_URL")?,
            server_port: env::var("AUTH_PORT")
                .unwrap_or_else(|_| "3001".to_string())
                .parse()
                .expect("AUTH_PORT must be a number"),
            server_addr: env::var("AUTH_HOST").unwrap_or_else(|_| "0.0.0.0".to_owned()),
            mode: env::var("MODE").unwrap_or_else(|_| "production".to_owned()),
            cors_origin: env::var("CORS_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:5173".to_owned()),
        })
    }
}

impl UrlConfig {
    pub fn from_env() -> Result<Self, env::VarError> {
        dotenvy::dotenv().ok();

        Ok(Self {
            database_url: env::var("URL_DATABASE_URL")?,
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_owned()),
            server_port: env::var("URL_PORT")
                .unwrap_or_else(|_| "3002".to_string())
                .parse()
                .expect("URL_PORT must be a number"),
            server_addr: env::var("URL_HOST").unwrap_or_else(|_| "0.0.0.0".to_owned()),
            mode: env::var("MODE").unwrap_or_else(|_| "production".to_owned()),
            cors_origin: env::var("CORS_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:5173".to_owned()),
        })
    }
}
