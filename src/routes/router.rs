use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    db::postgres::DbPool,
    handlers::url_handler::{get_all_urls, get_original_url, shorten_url},
};

use tower_http::trace::TraceLayer;

pub fn create_router(db: DbPool) -> Router {
    Router::new()
        .route("/", post(shorten_url).get(get_all_urls))
        .route("/{short_code}", get(get_original_url))
        .layer(TraceLayer::new_for_http())
        .with_state(db)
}
