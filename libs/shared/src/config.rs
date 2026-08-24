use std::env;
use std::marker::PhantomData;

pub struct AuthConfig {
    pub database_url: String,
    /// Read-replica URL. Falls back to `database_url` if not set (e.g. docker-compose).
    pub database_read_url: String,
    pub server_port: u16,
    pub mode: String,
    pub server_addr: String,
    pub cors_origin: String,
}

pub struct UrlConfig {
    pub database_url: String,
    /// Read-replica URL. Falls back to `database_url` if not set (e.g. docker-compose).
    pub database_read_url: String,
    pub redis_url: String,
    pub server_port: u16,
    pub server_addr: String,
    pub mode: String,
    pub cors_origin: String,
}

// Typestates
pub struct Missing;
pub struct Provided;

pub struct ConfigBuilder<DbState, PortState> {
    database_url: String,
    database_read_url: Option<String>,
    server_port: u16,
    server_addr: String,
    mode: String,
    cors_origin: String,
    redis_url: String,
    _marker: PhantomData<(DbState, PortState)>,
}

impl ConfigBuilder<Missing, Missing> {
    pub fn new() -> Self {
        Self {
            database_url: String::new(),
            database_read_url: None,
            server_port: 0,
            server_addr: "0.0.0.0".to_owned(),
            mode: "production".to_owned(),
            cors_origin: "http://localhost:8000".to_owned(),
            redis_url: "redis://127.0.0.1:6379".to_owned(),
            _marker: PhantomData,
        }
    }
}

impl<DbState, PortState> ConfigBuilder<DbState, PortState> {
    pub fn with_database_read_url(mut self, url: Option<String>) -> Self {
        if url.is_some() {
            self.database_read_url = url;
        }
        self
    }

    pub fn with_host(mut self, host: Option<String>) -> Self {
        if let Some(h) = host {
            self.server_addr = h;
        }
        self
    }

    pub fn with_mode(mut self, mode: Option<String>) -> Self {
        if let Some(m) = mode {
            self.mode = m;
        }
        self
    }

    pub fn with_cors_origin(mut self, origin: Option<String>) -> Self {
        if let Some(o) = origin {
            self.cors_origin = o;
        }
        self
    }

    pub fn with_redis_url(mut self, url: Option<String>) -> Self {
        if let Some(r) = url {
            self.redis_url = r;
        }
        self
    }
}

impl<PortState> ConfigBuilder<Missing, PortState> {
    pub fn with_database_url(self, url: String) -> ConfigBuilder<Provided, PortState> {
        ConfigBuilder {
            database_url: url,
            database_read_url: self.database_read_url,
            server_port: self.server_port,
            server_addr: self.server_addr,
            mode: self.mode,
            cors_origin: self.cors_origin,
            redis_url: self.redis_url,
            _marker: PhantomData,
        }
    }
}

impl<DbState> ConfigBuilder<DbState, Missing> {
    pub fn with_port(self, port: u16) -> ConfigBuilder<DbState, Provided> {
        ConfigBuilder {
            database_url: self.database_url,
            database_read_url: self.database_read_url,
            server_port: port,
            server_addr: self.server_addr,
            mode: self.mode,
            cors_origin: self.cors_origin,
            redis_url: self.redis_url,
            _marker: PhantomData,
        }
    }
}

impl ConfigBuilder<Provided, Provided> {
    pub fn build_auth_config(self) -> AuthConfig {
        let read_url = self
            .database_read_url
            .unwrap_or_else(|| self.database_url.clone());
        AuthConfig {
            database_url: self.database_url,
            database_read_url: read_url,
            server_port: self.server_port,
            server_addr: self.server_addr,
            mode: self.mode,
            cors_origin: self.cors_origin,
        }
    }

    pub fn build_url_config(self) -> UrlConfig {
        let read_url = self
            .database_read_url
            .unwrap_or_else(|| self.database_url.clone());
        UrlConfig {
            database_url: self.database_url,
            database_read_url: read_url,
            server_addr: self.server_addr,
            server_port: self.server_port,
            mode: self.mode,
            cors_origin: self.cors_origin,
            redis_url: self.redis_url,
        }
    }
}

impl AuthConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();

        let server_port = env::var("AUTH_PORT")
            .unwrap_or_else(|_| "3001".to_string())
            .parse::<u16>()?;

        let config = ConfigBuilder::new()
            .with_database_url(env::var("AUTH_DATABASE_URL")?)
            .with_port(server_port)
            .with_database_read_url(env::var("AUTH_DATABASE_READ_URL").ok())
            .with_host(env::var("AUTH_HOST").ok())
            .with_mode(env::var("MODE").ok())
            .with_cors_origin(env::var("CORS_ORIGIN").ok())
            .build_auth_config();

        Ok(config)
    }
}

impl UrlConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();

        let server_port = env::var("URL_PORT")
            .unwrap_or_else(|_| "3002".to_string())
            .parse::<u16>()?;

        let config = ConfigBuilder::new()
            .with_database_url(env::var("URL_DATABASE_URL")?)
            .with_port(server_port)
            .with_database_read_url(env::var("URL_DATABASE_READ_URL").ok())
            .with_host(env::var("URL_HOST").ok())
            .with_mode(env::var("MODE").ok())
            .with_cors_origin(env::var("CORS_ORIGIN").ok())
            .with_redis_url(env::var("REDIS_URL").ok())
            .build_url_config();

        Ok(config)
    }
}
