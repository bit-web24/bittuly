use crate::handlers::{
    delete_user, get_user_by_id, login, logout, request_signup_handler, update_user,
    verify_otp_handler,
};
use crate::health::health;
use crate::metrics::metrics;
use crate::models::AuthStateRef;
use axum::routing::{get, post};
use axum::{Router, middleware};
use shared::jwt::jwt_auth;

pub fn auth_routes() -> Router<AuthStateRef> {
    let protected = Router::new()
        .route(
            "/{user_id}",
            get(get_user_by_id).delete(delete_user).put(update_user),
        )
        .route("/logout", post(logout))
        .layer(middleware::from_fn(jwt_auth));

    let mut router = Router::new()
        .route("/signup", post(request_signup_handler)) // Step 1: send OTP
        .route("/verify-otp", post(verify_otp_handler)) // Step 2: verify OTP → create user + JWT
        .route("/login", post(login))
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .merge(protected)
        .fallback(|| async { axum::http::StatusCode::NOT_FOUND })
        .layer(middleware::from_fn(shared::metrics::track_metrics));

    // Only expose the OTP debug endpoint in development mode.
    // This lets load tests retrieve OTPs programmatically without real emails.
    // NEVER present in production builds.
    if std::env::var("MODE").unwrap_or_default() == "development" {
        router = router.route(
            "/debug/otp-store",
            get(crate::debug_handler::debug_otp_store_handler),
        );
    }

    router
}
