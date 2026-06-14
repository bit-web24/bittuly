use axum::{
    Router,
    handler::Handler,
    middleware,
    routing::{get, post},
};

use crate::{
    handlers::{delete_url_handler, get_all_urls, get_original_url, shorten_url},
    models::UrlStateRef,
};
use shared::jwt::jwt_auth;

pub fn url_routes() -> Router<UrlStateRef> {
    // API endpoints (require auth)
    let api_routes = Router::new()
        .route("/", post(shorten_url).get(get_all_urls))
        .route("/{id}", axum::routing::delete(delete_url_handler))
        .layer(middleware::from_fn(jwt_auth));

    Router::new()
        .nest("/api/urls", api_routes)
        // Public redirect (no auth required)
        .route("/{id}", get(get_original_url))
}
