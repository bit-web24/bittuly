use axum::{
    Router,
    routing::{get, post},
};
mod handlers;
use handlers::url_handler::{get_original_url, shorten_url};

pub fn create_router() -> Router {
    Router::new()
        .route("/", post(shorten_url))
        .route("/{short_code}", get(get_original_url))
}
