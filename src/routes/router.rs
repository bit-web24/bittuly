use axum::Router;

use crate::{
    db::postgres::DbPool,
    routes::{url_routes::url_routes, user_routes::user_routes},
};

use tower_http::trace::TraceLayer;

pub fn create_router(db: DbPool) -> Router {
    Router::new()
        .merge(url_routes())
        .nest("/users", user_routes())
        .layer(TraceLayer::new_for_http())
        .with_state(db)
}
