use redis::{aio::ConnectionManager, Client, RedisResult};

/// Shared Redis handle for the entire application.
///
/// `ConnectionManager` wraps a `MultiplexedConnection` and automatically
/// reconnects on network drops — essential for a distributed URL shortener
/// where Redis availability must be treated as a soft dependency.
///
/// It is `Clone + Send + Sync`, so it can live in `Arc<AppState>` and be
/// cloned cheaply into every Axum handler.
pub type RedisConn = ConnectionManager;

/// Connect to Redis and return a `ConnectionManager`.
///
/// Establishes one multiplexed TCP connection at startup and keeps it alive,
/// transparently reconnecting whenever the link is lost.
///
/// Requires the `connection-manager` + `tokio-comp` feature flags.
pub async fn init_redis(url: &str) -> RedisResult<RedisConn> {
    let client = Client::open(url)?;
    ConnectionManager::new(client).await
}
