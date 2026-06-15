use crate::{models::UrlStateRef, services as url_service};
use axum::{
    extract::{Extension, Json, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use shared::jwt::Claims;
use validator::Validate;

const DEFAULT_PAGE_LIMIT: i64 = 20;

#[derive(Deserialize)]
pub struct PaginationParams {
    /// Opaque cursor returned by the previous page response.
    pub cursor: Option<String>,
    /// Number of items per page (default 20, max 100).
    pub limit: Option<i64>,
    /// Search query to filter URLs.
    pub search: Option<String>,
}

#[derive(Serialize)]
pub struct UrlsPageResponse {
    pub urls: Vec<crate::models::Url>,
    pub next_cursor: Option<String>,
}

pub async fn get_all_urls(
    State(state): State<UrlStateRef>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    // Decode the hex cursor → i64
    let cursor: Option<i64> = match params.cursor.as_deref() {
        None | Some("") => None,
        Some(hex) => match i64::from_str_radix(hex, 16) {
            Ok(id) => Some(id),
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": "invalid cursor" })),
                )
                    .into_response();
            }
        },
    };

    let limit = params.limit.unwrap_or(DEFAULT_PAGE_LIMIT);
    let search = params.search.filter(|s| !s.trim().is_empty());

    match url_service::get_urls_page(&state.db, claims.sub, cursor, limit, search).await {
        Ok(page) => (
            StatusCode::OK,
            Json(UrlsPageResponse {
                urls: page.urls,
                next_cursor: page.next_cursor,
            }),
        )
            .into_response(),
        Err(err) => {
            tracing::error!("{:?}", err);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Deserialize, Validate)]
pub struct ShortenUrlRequest {
    #[validate(url)]
    pub original_url: String,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn shorten_url(
    State(state): State<UrlStateRef>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<ShortenUrlRequest>,
) -> impl IntoResponse {
    if let Err(errors) = body.validate() {
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(errors.to_string())).into_response();
    }
    match url_service::shorten_url(&state.db, &body.original_url, claims.sub, body.expires_at).await
    {
        Ok(Some(url)) => (StatusCode::CREATED, Json(url)).into_response(),
        Ok(None) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": "You have already shortened this URL" })),
        )
            .into_response(),
        Err(err) => {
            tracing::error!("{:?}", err);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_original_url(
    State(state): State<UrlStateRef>,
    Path(short_code): Path<String>,
) -> impl IntoResponse {
    // Singleflight Cache Coalescing (Thundering Herd Protection)
    // If 200,000 requests hit this exact line simultaneously, ONLY 1 will execute the async block.
    // The other 199,999 will safely wait in memory and instantly receive the exact same result!
    let result = state
        .l1_cache
        .get_with(short_code.clone(), async {
            let mut redis = state.redis.clone();

            // 1. Try Redis
            let cached: Option<String> = match redis.get::<_, Option<String>>(&short_code).await {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("redis get failed (falling back to db): {e}");
                    None
                }
            };

            if let Some(original_url) = cached {
                tracing::info!(short_code, "cache hit");
                // If it's in Redis, it's guaranteed valid because Redis handles TTL eviction natively.
                return Some((original_url, None));
            }

            // 2. Try DB
            tracing::info!(short_code, "cache miss");
            match url_service::get_original_url(&state.db, &short_code).await {
                Ok(Some((original_url, expires_at))) => {
                    // Populate Redis
                    let ttl: u64 = if let Some(exp) = expires_at {
                        let remaining = exp.signed_duration_since(chrono::Utc::now()).num_seconds();
                        if remaining <= 0 {
                            return Some((original_url, expires_at)); // Let caller handle expiration
                        }
                        remaining.try_into().unwrap_or(60 * 60 * 24) // if timestamp is known, but expired, use default TTL = 24 hours
                    } else {
                        60 * 60 * 24 // if timestamp is unknown, use default TTL = 24 hours
                    };

                    if let Err(e) = redis
                        .set_ex::<_, _, ()>(&short_code, &original_url, ttl)
                        .await
                    {
                        tracing::warn!("redis set_ex failed: {e}");
                    }

                    Some((original_url, expires_at))
                }
                _ => None, // Not found or error
            }
        })
        .await;

    // Process Result
    match result {
        Some((original_url, expires_at)) => {
            // Re-verify expiration in case it expired while sitting in the 3-second L1 microcache
            if let Some(exp) = expires_at
                && exp < chrono::Utc::now()
            {
                state.l1_cache.remove(&short_code).await;
                return StatusCode::GONE.into_response();
            }

            // Publish click event asynchronously via RabbitMQ
            if let Ok(conn) = state.rabbitmq.get().await
                && let Ok(channel) = conn.create_channel().await
            {
                let _ = channel
                    .basic_publish(
                        "",
                        "click_events_queue",
                        shared::lapin::options::BasicPublishOptions::default(),
                        short_code.as_bytes(),
                        shared::lapin::BasicProperties::default(),
                    )
                    .await;
            }

            Redirect::temporary(&original_url).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn delete_url_handler(
    State(state): State<UrlStateRef>,
    Extension(claims): Extension<Claims>,
    Path(url_id): Path<i64>,
) -> impl IntoResponse {
    match url_service::delete_url(&state.db, url_id, claims.sub).await {
        Ok(Some(short_code)) => {
            // Evict from Redis cache — non-fatal if Redis is unavailable
            let mut redis = state.redis.clone();
            if let Err(e) = redis.del::<_, ()>(&short_code).await {
                tracing::warn!("redis DEL {short_code} failed: {e}");
            }
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => {
            tracing::error!("{:?}", err);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
