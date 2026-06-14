use std::sync::Arc;

use axum::{
    Json,
    extract::{Extension, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Serialize;

use crate::{postgres::DbPool, state::AppState};

#[derive(Serialize)]
pub struct HealthResponse {
    /// "healthy" if all checks pass, "degraded" if any fail.
    pub status: &'static str,
    pub postgres: String,
    pub redis: String,
    /// Crate version from Cargo.toml at compile time.
    pub version: &'static str,
    /// Seconds since the server started.
    pub uptime_secs: u64,
}

/// `GET /health`
///
/// Public endpoint — no authentication required.
/// Returns 200 when all dependencies are reachable, 503 otherwise.
pub async fn health(
    State(db): State<DbPool>,
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    let uptime_secs = state.started_at.elapsed().as_secs();

    // ── Postgres: send a trivial query ───────────────────────────────────────
    let postgres = match sqlx::query("SELECT 1").execute(&db).await {
        Ok(_) => Ok("ok".to_owned()),
        Err(e) => {
            tracing::warn!("health check: postgres failed: {e}");
            Err(format!("error: {e}"))
        }
    };

    // ── Redis: PING ───────────────────────────────────────────────────────────
    let redis = {
        let mut conn = state.redis.clone();
        match redis::cmd("PING").query_async::<String>(&mut conn).await {
            Ok(_) => Ok("ok".to_owned()),
            Err(e) => {
                tracing::warn!("health check: redis failed: {e}");
                Err(format!("error: {e}"))
            }
        }
    };

    let healthy = postgres.is_ok() && redis.is_ok();
    let http_status = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        http_status,
        Json(HealthResponse {
            status: if healthy { "healthy" } else { "degraded" },
            postgres: postgres.unwrap_or_else(|e| e),
            redis: redis.unwrap_or_else(|e| e),
            version: env!("CARGO_PKG_VERSION"),
            uptime_secs,
        }),
    )
}
