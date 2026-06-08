use axum::{middleware, Router};
use axum::routing::{get, post};

use crate::{
    db::postgres::DbPool,
    handlers::user_handler::{create_user, delete_user, get_user_by_id, login, update_user},
    middlewares::jwt::jwt_auth,
};

pub fn user_routes() -> Router<DbPool> {
    let protected_routes = Router::new()
        .route(
            "/{user_id}",
            get(get_user_by_id).delete(delete_user).put(update_user),
        )
        .layer(middleware::from_fn(jwt_auth));

    Router::new()
        .route("/", post(create_user))
        .route("/login", post(login))
        .merge(protected_routes)
}
