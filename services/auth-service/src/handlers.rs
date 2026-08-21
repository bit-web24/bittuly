use crate::models::{
    AuthStateRef, CreateUserPayload, LoginPayload, UpdateUserPayload, VerifyOtpPayload,
};
use crate::services as user_service;
use axum::extract::{Extension, Json, Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;
use shared::jwt::{Claims, clear_token_cookies, set_token_cookies};
use uuid::Uuid;
use validator::{Validate, ValidationErrors};

/// Convert validator errors to a structured JSON-serializable map.
fn validation_error_response(errors: ValidationErrors) -> impl IntoResponse {
    let map: std::collections::HashMap<String, Vec<String>> = errors
        .field_errors()
        .iter()
        .map(|(field, errs)| {
            let messages = errs
                .iter()
                .map(|e| e.message.as_deref().unwrap_or("invalid value").to_string())
                .collect();
            (field.to_string(), messages)
        })
        .collect();
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({ "errors": map })),
    )
}

pub async fn login(
    State(state): State<AuthStateRef>,
    Json(payload): Json<LoginPayload>,
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return validation_error_response(errors).into_response();
    }
    // Route to read replica — login is a SELECT by email, safe for replicas.
    match user_service::login(&*state.read_repo, state.hasher.as_ref(), &payload.email, &payload.password).await {
        Ok(auth) => {
            let mut response = (StatusCode::OK, Json(auth.user)).into_response();
            if let Err(e) = set_token_cookies(&mut response, &auth.token, &auth.refresh_token) {
                tracing::error!("set_token_cookies: {:?}", e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            response
        }
        // Distinguish authentication failures from infrastructure failures
        Err(err)
            if err.to_string().contains("invalid credentials")
                || err.to_string().contains("not found") =>
        {
            StatusCode::UNAUTHORIZED.into_response()
        }
        Err(err) => {
            tracing::error!("login DB error: {:?}", err);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "service temporarily unavailable" })),
            )
                .into_response()
        }
    }
}

pub async fn logout() -> impl IntoResponse {
    let mut response = StatusCode::NO_CONTENT.into_response();
    clear_token_cookies(&mut response);
    response
}

pub async fn get_user_by_id(
    State(state): State<AuthStateRef>,
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

    // Route to read replica — SELECT by id (PK), safe for replicas.
    match user_service::get_user_by_id(&*state.read_repo, user_id).await {
        Ok(Some(user)) => (StatusCode::OK, Json(user)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "user not found" })),
        )
            .into_response(),
        Err(err) => {
            tracing::error!("get_user_by_id DB error: {:?}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "internal server error" })),
            )
                .into_response()
        }
    }
}

pub async fn update_user(
    State(state): State<AuthStateRef>,
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
        return validation_error_response(errors).into_response();
    }

    match user_service::update_user(&*state.repo, state.hasher.as_ref(), user_id, payload).await {
        Ok(auth) => {
            let mut response = (StatusCode::OK, Json(auth.user)).into_response();
            if let Err(e) = set_token_cookies(&mut response, &auth.token, &auth.refresh_token) {
                tracing::error!("set_token_cookies: {:?}", e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            response
        }
        Err(err) => {
            tracing::error!("update_user DB error: {:?}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "internal server error" })),
            )
                .into_response()
        }
    }
}

pub async fn delete_user(
    State(state): State<AuthStateRef>,
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

    match user_service::delete_user(&*state.repo, user_id).await {
        Ok(_) => {
            // Publish to RabbitMQ
            let payload = serde_json::json!({ "user_id": user_id }).to_string();
            if let Ok(conn) = state.rabbitmq.get().await
                && let Ok(channel) = conn.create_channel().await
            {
                use std::collections::HashMap;

                let mut carrier = HashMap::new();
                shared::telemetry::inject_context(&mut carrier);

                // Convert carrier to AMQP headers
                let mut amqp_headers = shared::lapin::types::FieldTable::default();
                for (k, v) in carrier {
                    amqp_headers.insert(
                        k.into(),
                        shared::lapin::types::AMQPValue::LongString(v.into()),
                    );
                }
                let props = shared::lapin::BasicProperties::default().with_headers(amqp_headers);

                let _ = channel
                    .basic_publish(
                        "", // default exchange
                        "user_deleted_queue",
                        shared::lapin::options::BasicPublishOptions::default(),
                        payload.as_bytes(),
                        props,
                    )
                    .await;
            }

            StatusCode::NO_CONTENT.into_response()
        }
        Err(err) => {
            tracing::error!("delete_user DB error: {:?}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "internal server error" })),
            )
                .into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// OTP signup flow handlers
// ---------------------------------------------------------------------------

/// POST /users/signup
/// Validates payload, sends OTP email, returns { pending_token } (no user created yet).
pub async fn request_signup_handler(
    State(state): State<AuthStateRef>,
    Json(payload): Json<CreateUserPayload>,
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(errors.to_string())).into_response();
    }
    match user_service::request_signup(&*state.repo, state.hasher.as_ref(), payload).await {
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
    State(state): State<AuthStateRef>,
    Json(payload): Json<VerifyOtpPayload>,
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(errors.to_string())).into_response();
    }
    match user_service::verify_otp(&*state.repo, state.hasher.as_ref(), &payload.pending_token, &payload.otp).await {
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
