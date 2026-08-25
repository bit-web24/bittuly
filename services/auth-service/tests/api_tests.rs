use auth_service::mock::MockUserRepo;
use auth_service::models::{AuthState, User};
use auth_service::routes::auth_routes;
use axum::http::StatusCode;
use axum_test::TestServer;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use shared::deadpool_lapin::{Config, Runtime};
use std::sync::Arc;
use std::sync::OnceLock;

use auth_service::password_hasher::PlainTextHasher;

static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

async fn setup_app() -> TestServer {
    unsafe {
        std::env::set_var("JWT_SECRET", "super-secret-test-key-12345");
        std::env::set_var("MODE", "development");
    }

    let repo = Arc::new(MockUserRepo::new());
    let publisher = Arc::new(shared::rabbitmq::MockEventPublisher);
    let hasher = Arc::new(PlainTextHasher);

    let prometheus_handle = PROMETHEUS_HANDLE
        .get_or_init(|| {
            PrometheusBuilder::new()
                .install_recorder()
                .expect("failed to install recorder in test")
        })
        .clone();

    let auth_state = Arc::new(AuthState::new(
        repo.clone(),
        repo,
        publisher,
        hasher,
        prometheus_handle,
    ));

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

    let res = server.post("/signup").json(&signup_payload).await;
    res.assert_status_ok();

    let pending_token = res.json::<serde_json::Value>()["pending_token"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(!pending_token.is_empty());

    // 2. Fetch OTP from memory
    let otp = auth_service::otp_store::get_all()
        .into_iter()
        .find(|o| o.email == "api@example.com")
        .unwrap()
        .otp;

    // 3. Verify OTP
    let verify_payload = serde_json::json!({
        "pending_token": pending_token,
        "otp": otp
    });

    let res_verify = server.post("/verify-otp").json(&verify_payload).await;
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
    let otp = auth_service::otp_store::get_all()
        .into_iter()
        .find(|o| o.email == "login@example.com")
        .unwrap()
        .otp;
    server
        .post("/verify-otp")
        .json(&serde_json::json!({"pending_token": token, "otp": otp}))
        .await;

    // Attempt Login
    let res = server
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
