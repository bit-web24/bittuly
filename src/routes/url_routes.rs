use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    db::postgres::DbPool,
    handlers::url_handler::{get_all_urls, get_original_url, shorten_url},
};

pub fn url_routes() -> Router<DbPool> {
    Router::new()
        .route("/", post(shorten_url).get(get_all_urls))
        .route("/{short_code}", get(get_original_url))
}
