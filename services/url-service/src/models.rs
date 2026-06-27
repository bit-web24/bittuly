use std::sync::Arc;
use std::time::Duration;

use crate::repo_trait::UrlRepo;
use chrono::DateTime;
use chrono::Utc;
use metrics_exporter_prometheus::PrometheusHandle;
use moka::future::Cache;
use serde::Serialize;
use shared::redis::RedisConn;

use tokio::time::Instant;
use uuid::Uuid;

pub type CachedUrlResult = Option<(String, Option<DateTime<Utc>>)>;

pub struct UrlState {
    pub rabbitmq: shared::deadpool_lapin::Pool,
    /// Write pool — always the CNPG primary.
    pub repo: Arc<dyn UrlRepo>,
    /// Read pool — CNPG replica(s). Falls back to primary if replicas unavailable.
    pub read_repo: Arc<dyn UrlRepo>,
    pub redis: RedisConn,
    #[allow(dead_code)]
    pub started_at: Instant,
    #[allow(dead_code)]
    pub cors_origin: String,
    pub l1_cache: Cache<String, CachedUrlResult>,
    pub prometheus_handle: PrometheusHandle,
}

impl UrlState {
    pub fn new(
        rabbitmq: shared::deadpool_lapin::Pool,
        redis: RedisConn,
        repo: Arc<dyn UrlRepo>,
        read_repo: Arc<dyn UrlRepo>,
        cors_origin: String,
        prometheus_handle: PrometheusHandle,
    ) -> Self {
        let l1_cache = Cache::builder()
            .max_capacity(1_000_000)
            .time_to_live(Duration::from_secs(3))
            .build();

        Self {
            rabbitmq,
            repo,
            read_repo,
            redis,
            started_at: Instant::now(),
            cors_origin,
            l1_cache,
            prometheus_handle,
        }
    }
}
pub type UrlStateRef = Arc<UrlState>;

#[derive(sqlx::FromRow, Serialize, serde::Deserialize, Clone, Debug)]
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
