use axum::Router;
use axum::routing::get;

use crate::{
    db::postgres::DbPool,
    handlers::debug_handler::debug_otp_store_handler,
    routes::{url_routes::url_routes, user_routes::user_routes},
};

use tower_http::trace::TraceLayer;

pub fn create_router(db: DbPool, mode: &str) -> Router {
    let mut router = Router::new()
        .merge(url_routes())
        .nest("/users", user_routes());

    if mode == "development" {
        router = router.route("/debug/otp-store", get(debug_otp_store_handler));
        tracing::warn!("[DEV] GET /debug/otp-store is enabled — disable in production");
    }

    router
        .layer(TraceLayer::new_for_http())
        .with_state(db)
}

