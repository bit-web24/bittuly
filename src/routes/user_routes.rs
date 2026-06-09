use axum::{middleware, Router};
use axum::routing::{get, post};

use crate::{
    db::postgres::DbPool,
    handlers::user_handler::{
        create_user, delete_user, get_user_by_id, login, logout, update_user,
        request_signup_handler, verify_otp_handler,
    },
    middlewares::jwt::jwt_auth,
};

pub fn user_routes() -> Router<DbPool> {
    let protected = Router::new()
        .route("/{user_id}", get(get_user_by_id).delete(delete_user).put(update_user))
        .route("/logout", post(logout))
        .layer(middleware::from_fn(jwt_auth));

    Router::new()
        .route("/signup", post(request_signup_handler))  // Step 1: send OTP
        .route("/verify-otp", post(verify_otp_handler))  // Step 2: verify OTP → create user + JWT
        .route("/direct-signup", post(create_user))       // Legacy: direct signup (no OTP)
        .route("/login", post(login))
        .merge(protected)
}

