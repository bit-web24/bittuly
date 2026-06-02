use std::env;

pub struct Settings {
    pub database_url: String,
    pub server_addr: String,
}

impl Settings {
    pub fn from_env() -> Result<Self, env::VarError> {
        let database_url = env::var("DATABASE_URL")?;
        let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
        let port = env::var("PORT").unwrap_or_else(|_| "3000".to_owned());

        Ok(Self {
            database_url,
            server_addr: format!("{host}:{port}"),
        })
    }
}
