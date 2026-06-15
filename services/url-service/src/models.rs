use std::sync::Arc;
use std::time::Duration;

use chrono::DateTime;
use chrono::Utc;
use moka::future::Cache;
use serde::Serialize;
use shared::redis::RedisConn;
use sqlx::PgPool;

use tokio::time::Instant;
use uuid::Uuid;

pub struct UrlState {
    pub rabbitmq: shared::deadpool_lapin::Pool,
    pub db: PgPool,
    pub redis: RedisConn,
    #[allow(dead_code)]
    pub started_at: Instant,
    #[allow(dead_code)]
    pub cors_origin: String,
    pub l1_cache: Cache<String, Option<(String, Option<DateTime<Utc>>)>>,
}

impl UrlState {
    pub fn new(
        rabbitmq: shared::deadpool_lapin::Pool,
        redis: RedisConn,
        db: PgPool,
        cors_origin: String,
    ) -> Self {
        let l1_cache = Cache::builder()
            .max_capacity(1_000_000)
            .time_to_live(Duration::from_secs(3))
            .build();

        Self {
            rabbitmq,
            db,
            redis,
            started_at: Instant::now(),
            cors_origin,
            l1_cache,
        }
    }
}
pub type UrlStateRef = Arc<UrlState>;

#[derive(sqlx::FromRow, Serialize)]
pub struct Url {
    #[serde(rename = "id")]
    pub url_id: i64,
    pub short_code: String,
    pub original_url: String,
    pub user_id: Uuid,
    pub click_count: i64,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
