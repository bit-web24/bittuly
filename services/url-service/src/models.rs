use std::sync::Arc;

use chrono::DateTime;
use chrono::Utc;
use serde::Serialize;
use shared::redis::RedisConn;
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::Instant;
use uuid::Uuid;

pub type ClickSender = UnboundedSender<String>;

pub struct UrlState {
    pub db: PgPool,
    pub tx: ClickSender,
    pub redis: RedisConn,
    #[allow(dead_code)]
    pub started_at: Instant,
    pub cors_origin: String,
}

impl UrlState {
    pub fn new(tx: ClickSender, redis: RedisConn, db: PgPool, cors_origin: String) -> Self {
        Self {
            db,
            tx,
            redis,
            started_at: Instant::now(),
            cors_origin,
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
