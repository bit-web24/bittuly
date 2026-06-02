use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    db::postgres::DbPool,
    handlers::url_handler::{get_original_url, shorten_url},
};

pub fn create_router(db: DbPool) -> Router {
    Router::new()
        .route("/", post(shorten_url))
        .route("/{short_code}", get(get_original_url))
        .with_state(db)
}
