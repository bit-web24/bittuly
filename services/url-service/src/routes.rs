use axum::{
    Router, middleware,
    routing::{get, post},
};

use crate::{
    handlers::{delete_url_handler, get_all_urls, get_original_url, shorten_url},
    health::health,
    metrics::metrics,
    models::UrlStateRef,
};
use shared::jwt::jwt_auth;

pub fn url_routes() -> Router<UrlStateRef> {
    // API endpoints (require auth)
    let protected = Router::new()
        .route("/api/urls", post(shorten_url).get(get_all_urls))
        .route("/api/urls/{id}", axum::routing::delete(delete_url_handler))
        .layer(middleware::from_fn(jwt_auth));

    Router::new()
        .merge(protected)
        .route("/api/urls/health", get(health))
        .route("/api/urls/metrics", get(metrics))
        .route("/{id}", get(get_original_url))
        .fallback(|| async { axum::http::StatusCode::NOT_FOUND })
        .layer(axum::middleware::from_fn(shared::metrics::track_metrics))
}
