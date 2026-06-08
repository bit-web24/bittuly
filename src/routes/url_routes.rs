use axum::{
    middleware,
    Router,
    routing::{get, post},
};

use crate::{
    db::postgres::DbPool,
    handlers::url_handler::{get_all_urls, get_original_url, shorten_url},
    middlewares::jwt::jwt_auth,
};

pub fn url_routes() -> Router<DbPool> {
    let protected = Router::new()
        .route("/", post(shorten_url).get(get_all_urls))
        .layer(middleware::from_fn(jwt_auth));

    Router::new()
        .merge(protected)
        .route("/{short_code}", get(get_original_url))
}
