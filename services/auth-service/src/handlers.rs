use crate::models::{CreateUserPayload, LoginPayload, UpdateUserPayload, VerifyOtpPayload};
use crate::service as user_service;
use axum::extract::{Extension, Json, Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;
use shared::jwt::{Claims, clear_token_cookies, set_token_cookies};
use shared::postgres::DbPool;
use uuid::Uuid;
use validator::Validate;

pub async fn create_user(
    State(db): State<DbPool>,
    Json(payload): Json<CreateUserPayload>,
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(errors.to_string())).into_response();
    }
    match user_service::create_user(&db, payload).await {
        Ok(auth) => {
            let mut response = (StatusCode::CREATED, Json(auth.user)).into_response();
            if let Err(e) = set_token_cookies(&mut response, &auth.token, &auth.refresh_token) {
                tracing::error!("set_token_cookies: {:?}", e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            response
        }
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(err.to_string())).into_response(),
    }
}

pub async fn login(
    State(db): State<DbPool>,
    Json(payload): Json<LoginPayload>,
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(errors.to_string())).into_response();
    }
    match user_service::login(&db, &payload.email, &payload.password).await {
        Ok(auth) => {
            let mut response = (StatusCode::OK, Json(auth.user)).into_response();
            if let Err(e) = set_token_cookies(&mut response, &auth.token, &auth.refresh_token) {
                tracing::error!("set_token_cookies: {:?}", e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            response
        }
        Err(_) => StatusCode::UNAUTHORIZED.into_response(),
    }
}

pub async fn logout() -> impl IntoResponse {
    let mut response = StatusCode::NO_CONTENT.into_response();
    clear_token_cookies(&mut response);
    response
}

pub async fn get_user_by_id(
    State(db): State<DbPool>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid user_id").into_response(),
    };

    if user_id != claims.sub {
        return StatusCode::FORBIDDEN.into_response();
    }

    match user_service::get_user_by_id(&db, user_id).await {
        Ok(Some(user)) => (StatusCode::OK, Json(user)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(err.to_string())).into_response(),
    }
}

pub async fn update_user(
    State(db): State<DbPool>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<String>,
    Json(payload): Json<UpdateUserPayload>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid user_id").into_response(),
    };

    if user_id != claims.sub {
        return StatusCode::FORBIDDEN.into_response();
    }

    if let Err(errors) = payload.validate() {
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(errors.to_string())).into_response();
    }

    match user_service::update_user(&db, user_id, payload).await {
        Ok(auth) => {
            let mut response = (StatusCode::OK, Json(auth.user)).into_response();
            if let Err(e) = set_token_cookies(&mut response, &auth.token, &auth.refresh_token) {
                tracing::error!("set_token_cookies: {:?}", e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            response
        }
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(err.to_string())).into_response(),
    }
}

pub async fn delete_user(
    State(db): State<DbPool>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid user_id").into_response(),
    };

    if user_id != claims.sub {
        return StatusCode::FORBIDDEN.into_response();
    }

    match user_service::delete_user(&db, user_id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(err.to_string())).into_response(),
    }
}

// ---------------------------------------------------------------------------
// OTP signup flow handlers
// ---------------------------------------------------------------------------

/// POST /users/signup
/// Validates payload, sends OTP email, returns { pending_token } (no user created yet).
pub async fn request_signup_handler(
    State(db): State<DbPool>,
    Json(payload): Json<CreateUserPayload>,
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(errors.to_string())).into_response();
    }
    match user_service::request_signup(&db, payload).await {
        Ok(pending_token) => (
            StatusCode::OK,
            Json(json!({ "pending_token": pending_token })),
        )
            .into_response(),
        Err(err) if err.to_string() == "email already registered" => (
            StatusCode::CONFLICT,
            Json(json!({ "error": err.to_string() })),
        )
            .into_response(),
        Err(err) => {
            tracing::error!("request_signup: {:?}", err);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// POST /users/verify-otp
/// Verifies OTP, creates the real user, sets auth cookies, returns user.
pub async fn verify_otp_handler(
    State(db): State<DbPool>,
    Json(payload): Json<VerifyOtpPayload>,
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(errors.to_string())).into_response();
    }
    match user_service::verify_otp(&db, &payload.pending_token, &payload.otp).await {
        Ok(auth) => {
            let mut response = (StatusCode::CREATED, Json(auth.user)).into_response();
            if let Err(e) = set_token_cookies(&mut response, &auth.token, &auth.refresh_token) {
                tracing::error!("set_token_cookies: {:?}", e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            response
        }
        Err(err) if err.to_string() == "invalid OTP" => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "invalid or expired OTP" })),
        )
            .into_response(),
        Err(err) => {
            tracing::error!("verify_otp: {:?}", err);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
