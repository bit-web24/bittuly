use std::sync::Arc;

use axum::Router;
use axum::Extension;
use axum::http::{HeaderValue, Method, header};
use axum::routing::get;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::app::state::AppState;
use crate::{
    db::postgres::DbPool,
    handlers::debug_handler::debug_otp_store_handler,
    routes::{url_routes::url_routes, user_routes::user_routes},
};

pub fn create_router(db: DbPool, mode: &str, cors_origin: &str, state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(
            cors_origin
                .parse::<HeaderValue>()
                .expect("Invalid CORS_ORIGIN value"),
        )
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true);

    let mut router = Router::new()
        .merge(url_routes())
        .nest("/users", user_routes())
        // Inject AppState as an Extension so handlers can access tx
        // without changing the primary state type (DbPool)
        .layer(Extension(state));

    if mode == "development" {
        router = router.route("/debug/otp-store", get(debug_otp_store_handler));
        tracing::warn!("[DEV] GET /debug/otp-store is enabled — disable in production");
    }

    router
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(db)
}
