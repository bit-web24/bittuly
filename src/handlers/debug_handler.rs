use axum::{Json, response::IntoResponse};
use crate::utils::otp_store;

/// GET /debug/otp-store
/// Returns all stored OTP entries. Only available when MODE=development.
pub async fn debug_otp_store_handler() -> impl IntoResponse {
    Json(otp_store::get_all())
}
