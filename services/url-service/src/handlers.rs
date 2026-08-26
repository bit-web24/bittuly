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

    // Route to read replica — this is a pure SELECT, safe for replicas.
    match url_service::get_urls_page(&*state.repo, claims.sub, cursor, limit, search).await {
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

#[derive(serde::Deserialize, serde::Serialize, Validate)]
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
    match url_service::shorten_url(
        &*state.repo,
        &body.original_url,
        claims.sub,
        body.expires_at,
    )
    .await
    {
        Ok(Some(url)) => {
            metrics::counter!("links_shortened").increment(1);
            (StatusCode::CREATED, Json(url)).into_response()
        }
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
    let result = state
        .l1_cache
        .try_get_with(short_code.clone(), async {
            let mut redis = state.redis.clone();

            let cached: Option<String> = {
                // Ensure we don't hold a !Send tracing guard across an await point
                match redis.get::<_, Option<String>>(&short_code).await {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!("redis get failed (falling back to db): {e}");
                        None
                    }
                }
            };

            if let Some(original_url) = cached {
                metrics::counter!("cache_hits").increment(1);
                return Ok::<_, axum::http::StatusCode>(Some((original_url, None)));
            }

            // Route to read replica — this is a pure SELECT by short_code.
            metrics::counter!("cache_misses").increment(1);
            match url_service::get_original_url(&*state.repo, &short_code).await {
                Ok(Some((original_url, expires_at))) => {
                    // 3. Update Redis cache in background
                    let original_url_clone = original_url.clone();
                    let short_code_clone = short_code.clone();
                    tokio::spawn(async move {
                        if let Some(exp) = expires_at {
                            let ttl = exp.timestamp() - chrono::Utc::now().timestamp();
                            if ttl > 0 {
                                let _: () = redis
                                    .set_ex(&short_code_clone, original_url_clone, ttl as u64)
                                    .await
                                    .unwrap_or_default();
                            }
                        } else {
                            let _: () = redis
                                .set(&short_code_clone, original_url_clone)
                                .await
                                .unwrap_or_default();
                        }
                    });
                    Ok(Some((original_url, expires_at)))
                }
                Ok(None) => Ok(None), // True 404 Not Found
                Err(e) => {
                    tracing::error!("DB error during redirect: {:?}", e);
                    Err(axum::http::StatusCode::SERVICE_UNAVAILABLE)
                }
            }
        })
        .await;

    // Process Result
    match result {
        Ok(Some((original_url, expires_at))) => {
            // Re-verify expiration in case it expired while sitting in the 3-second L1 microcache
            if let Some(exp) = expires_at
                && exp < chrono::Utc::now()
            {
                state.l1_cache.remove(&short_code).await;
                return StatusCode::GONE.into_response();
            }

            // Publish click event asynchronously via RabbitMQ
            let publisher = state.publisher.clone();
            let short_code_clone = short_code.clone();
            tokio::spawn(async move {
                if publisher
                    .publish("click_events_queue", short_code_clone.as_bytes())
                    .await
                    .is_ok()
                {
                    metrics::counter!("rabbit_mq_events_published", "queue" => "click_events_queue")
                        .increment(1);
                }
            });

            Redirect::temporary(&original_url).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            // Moka wraps the error in Arc
            let status = *e;
            status.into_response()
        }
    }
}

pub async fn delete_url_handler(
    State(state): State<UrlStateRef>,
    Extension(claims): Extension<Claims>,
    Path(url_id): Path<i64>,
) -> impl IntoResponse {
    match url_service::delete_url(&*state.repo, url_id, claims.sub).await {
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
