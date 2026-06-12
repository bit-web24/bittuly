use axum::{
    Router,
    handler::Handler,
    middleware,
    routing::{get, post},
};

use crate::{
    db::postgres::DbPool,
    handlers::url_handler::{delete_url_handler, get_all_urls, get_original_url, shorten_url},
    middlewares::jwt::jwt_auth,
};

pub fn url_routes() -> Router<DbPool> {
    // POST / and GET / both require auth
    let protected_root = Router::new()
        .route("/", post(shorten_url).get(get_all_urls))
        .layer(middleware::from_fn(jwt_auth));

    Router::new()
        .merge(protected_root)
        // GET /{id} is public; DELETE /{id} requires auth — applied at handler level
        .route(
            "/{id}",
            get(get_original_url)
                .delete(delete_url_handler.layer(middleware::from_fn(jwt_auth))),
        )
}
