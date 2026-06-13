use crate::{
    app::state::AppState, db::postgres::DbPool, middlewares::jwt::Claims, services::url_service,
};
use axum::{
    extract::{Extension, Json, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

const DEFAULT_PAGE_LIMIT: i64 = 20;

#[derive(Deserialize)]
pub struct PaginationParams {
    /// Opaque cursor returned by the previous page response.
    pub cursor: Option<String>,
    /// Number of items per page (default 20, max 100).
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct UrlsPageResponse {
    pub urls: Vec<crate::models::Url>,
    pub next_cursor: Option<String>,
}

pub async fn get_all_urls(
    State(db): State<DbPool>,
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
                    .into_response()
            }
        },
    };

    let limit = params.limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    match url_service::get_urls_page(&db, claims.sub, cursor, limit).await {
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
}



pub async fn shorten_url(
    State(db): State<DbPool>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<ShortenUrlRequest>,
) -> impl IntoResponse {
    if let Err(errors) = body.validate() {
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(errors.to_string())).into_response();
    }
    match url_service::shorten_url(&db, &body.original_url, claims.sub).await {
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
    State(db): State<DbPool>,
    Extension(state): Extension<Arc<AppState>>,
    Path(short_code): Path<String>,
) -> impl IntoResponse {
    let mut redis = state.redis.clone();

    // Cache lookup
    let cached: Option<String> = match redis.get::<_, Option<String>>(&short_code).await {
        Ok(v) => v,
        Err(e) => {
            // Redis unavailable — non-fatal, fall through to DB
            tracing::warn!("redis get failed (falling back to db): {e}");
            None
        }
    };

    if let Some(original_url) = cached {
        tracing::info!(short_code, "cache hit");
        if let Err(e) = state.tx.send(short_code) {
            tracing::warn!("click channel send failed: {e}");
        }
        return Redirect::temporary(&original_url).into_response();
    }

    // ── 2. Cache miss — query DB ───────────────────────────────────────────
    tracing::info!(short_code, "cache miss");
    match url_service::get_original_url(&db, &short_code).await {
        Ok(Some(original_url)) => {
            // Populate cache with 24 h TTL, non-fatal if Redis is down
            if let Err(e) = redis
                .set_ex::<_, _, ()>(&short_code, &original_url, 60 * 60 * 24)
                .await
            {
                tracing::warn!("redis set_ex failed: {e}");
            }

            if let Err(e) = state.tx.send(short_code) {
                tracing::warn!("click channel send failed: {e}");
            }
            Redirect::temporary(&original_url).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => {
            tracing::error!("{:?}", err);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_url_handler(
    State(db): State<DbPool>,
    Extension(state): Extension<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(url_id): Path<i64>,
) -> impl IntoResponse {
    match url_service::delete_url(&db, url_id, claims.sub).await {
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
