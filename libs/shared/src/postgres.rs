use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;

pub type DbPool = PgPool;

/// Initialize a PostgreSQL connection pool.
///
/// Pool size and timeouts are configurable via environment variables:
/// - `DB_MAX_CONNECTIONS` (default: 20)
/// - `DB_CONNECT_TIMEOUT_SECS` (default: 5)
/// - `DB_IDLE_TIMEOUT_SECS` (default: 600)
/// - `DB_MAX_LIFETIME_SECS` (default: 1800)
pub async fn init_pg_pool(database_url: &str) -> Result<DbPool, sqlx::Error> {
    let max_connections: u32 = std::env::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);

    let connect_timeout: u64 = std::env::var("DB_CONNECT_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);

    let idle_timeout: u64 = std::env::var("DB_IDLE_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(600);

    let max_lifetime: u64 = std::env::var("DB_MAX_LIFETIME_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1800);

    PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(connect_timeout))
        .idle_timeout(Duration::from_secs(idle_timeout))
        .max_lifetime(Duration::from_secs(max_lifetime))
        .connect(database_url)
        .await
}
