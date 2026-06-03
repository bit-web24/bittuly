use axum::Router;
use axum::routing::{get, post};

use crate::db::postgres::DbPool;
use crate::handlers::user_handler::{create_user, delete_user, get_user_by_id, update_user};

pub fn user_routes() -> Router<DbPool> {
    Router::new().route("/", post(create_user)).route(
        "/{user_id}",
        get(get_user_by_id).delete(delete_user).put(update_user),
    )
}
