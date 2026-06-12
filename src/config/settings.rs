use std::env;

pub struct Settings {
    pub database_url: String,
    pub redis_url: String,
    pub server_addr: String,
    /// "development" | "production"
    pub mode: String,
    /// Allowed CORS origin, e.g. "http://localhost:5173"
    pub cors_origin: String,
}

impl Settings {
    pub fn from_env() -> Result<Self, env::VarError> {
        dotenvy::dotenv().ok();

        let database_url = env::var("DATABASE_URL")?;
        let redis_url = env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_owned());
        let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
        let port = env::var("PORT").unwrap_or_else(|_| "3000".to_owned());
        let mode = env::var("MODE").unwrap_or_else(|_| "production".to_owned());
        let cors_origin = env::var("CORS_ORIGIN")
            .unwrap_or_else(|_| "http://localhost:5173".to_owned());

        Ok(Self {
            database_url,
            redis_url,
            server_addr: format!("{host}:{port}"),
            mode,
            cors_origin,
        })
    }
}
