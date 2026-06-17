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
    match user_service::login(&*state.repo, &payload.email, &payload.password).await {
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

    match user_service::get_user_by_id(&*state.repo, user_id).await {
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

    match user_service::update_user(&*state.repo, user_id, payload).await {
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
                let _ = channel
                    .basic_publish(
                        "", // default exchange
                        "user_deleted_queue",
                        shared::lapin::options::BasicPublishOptions::default(),
                        payload.as_bytes(),
                        shared::lapin::BasicProperties::default(),
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
    match user_service::request_signup(&*state.repo, payload).await {
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
    match user_service::verify_otp(&*state.repo, &payload.pending_token, &payload.otp).await {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockUserRepo;
    use crate::models::{AuthState, User};
    use crate::routes::auth_routes;
    use axum_test::TestServer;
    use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
    use shared::deadpool_lapin::{Config, Runtime};
    use std::sync::Arc;

    use std::sync::OnceLock;

    static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

    async fn setup_app() -> TestServer {
        unsafe {
            std::env::set_var("JWT_SECRET", "super-secret-test-key-12345");
            std::env::set_var("MODE", "development");
        }

        let repo = Arc::new(MockUserRepo::new());
        let rabbit_cfg = Config::default();
        let rabbitmq = rabbit_cfg.create_pool(Some(Runtime::Tokio1)).unwrap();

        let prometheus_handle = PROMETHEUS_HANDLE
            .get_or_init(|| {
                PrometheusBuilder::new()
                    .install_recorder()
                    .expect("failed to install recorder in test")
            })
            .clone();

        let auth_state = Arc::new(AuthState::new(repo, rabbitmq, prometheus_handle));

        let app = auth_routes().with_state(auth_state);
        TestServer::new(app)
    }

    #[tokio::test]
    async fn test_api_signup_and_verify_flow() {
        let server = setup_app().await;

        // 1. Request Signup
        let signup_payload = serde_json::json!({
            "username": "apiuser",
            "email": "api@example.com",
            "password": "password123"
        });

        let mut res = server.post("/signup").json(&signup_payload).await;
        res.assert_status_ok();

        let pending_token = res.json::<serde_json::Value>()["pending_token"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(!pending_token.is_empty());

        // 2. Fetch OTP from memory
        let otp = crate::otp_store::get_all()
            .into_iter()
            .find(|o| o.email == "api@example.com")
            .unwrap()
            .otp;

        // 3. Verify OTP
        let verify_payload = serde_json::json!({
            "pending_token": pending_token,
            "otp": otp
        });

        let mut res_verify = server.post("/verify-otp").json(&verify_payload).await;
        res_verify.assert_status(StatusCode::CREATED);

        // Ensure cookies were set
        res_verify.assert_contains_cookie("access_token");
        res_verify.assert_contains_cookie("refresh_token");

        let user = res_verify.json::<User>();
        assert_eq!(user.email, "api@example.com");
        assert_eq!(user.username, "apiuser");
    }

    #[tokio::test]
    async fn test_api_login_success() {
        let server = setup_app().await;

        // Seed user via signup flow first
        let payload = serde_json::json!({"username": "user", "email": "login@example.com", "password": "password123"});
        let signup = server.post("/signup").json(&payload).await;
        let token = signup.json::<serde_json::Value>()["pending_token"]
            .as_str()
            .unwrap()
            .to_string();
        let otp = crate::otp_store::get_all()
            .into_iter()
            .find(|o| o.email == "login@example.com")
            .unwrap()
            .otp;
        server
            .post("/verify-otp")
            .json(&serde_json::json!({"pending_token": token, "otp": otp}))
            .await;

        // Attempt Login
        let mut res = server
            .post("/login")
            .json(&serde_json::json!({
                "email": "login@example.com",
                "password": "password123"
            }))
            .await;

        res.assert_status_ok();
        res.assert_contains_cookie("access_token");
    }

    #[tokio::test]
    async fn test_api_validation_error() {
        let server = setup_app().await;

        // Missing password, invalid email
        let payload = serde_json::json!({
            "username": "us",
            "email": "not-an-email"
        });

        let res = server.post("/signup").json(&payload).await;
        res.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    }
}
