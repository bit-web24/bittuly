use axum::http::StatusCode;
use axum_test::TestServer;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use shared::deadpool_lapin::{Config, Runtime};
use shared::jwt::Claims;
use std::sync::{Arc, OnceLock};
use url_service::handlers::ShortenUrlRequest;
use url_service::mock::MockUrlRepo;
use url_service::models::Url;
use url_service::models::UrlState;
use url_service::routes::url_routes;

static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

async fn setup_app() -> TestServer {
    unsafe {
        std::env::set_var("JWT_SECRET", "super-secret-test-key-12345");
        std::env::set_var("MODE", "development");
    }

    let repo = Arc::new(MockUrlRepo::new());
    let publisher = Arc::new(shared::rabbitmq::MockEventPublisher);
    let redis = shared::redis::init_redis("redis://127.0.0.1")
        .await
        .unwrap();

    let prometheus_handle = PROMETHEUS_HANDLE
        .get_or_init(|| {
            PrometheusBuilder::new()
                .install_recorder()
                .expect("failed to install recorder in test")
        })
        .clone();

    let state = Arc::new(UrlState::new(
        publisher,
        redis,
        repo.clone(),
        repo,
        "http://localhost:8000".to_string(),
        prometheus_handle,
    ));

    let app = url_routes().with_state(state);
    TestServer::new(app)
}

fn generate_test_token() -> String {
    let claims = Claims {
        sub: uuid::Uuid::new_v4(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        token_type: "access".to_string(),
    };
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret("super-secret-test-key-12345".as_ref()),
    )
    .unwrap()
}

#[tokio::test]
async fn test_api_shorten_url_success() {
    let server = setup_app().await;
    let token = generate_test_token();

    let payload = ShortenUrlRequest {
        original_url: "https://rust-lang.org".to_string(),
        expires_at: None,
    };

    let res = server
        .post("/api/urls")
        .add_cookie(cookie::Cookie::new("access_token", token))
        .json(&payload)
        .await;

    res.assert_status(StatusCode::CREATED);

    let url = res.json::<Url>();
    assert_eq!(url.original_url, "https://rust-lang.org");
    assert!(!url.short_code.is_empty());
}

#[tokio::test]
async fn test_api_get_original_url() {
    let server = setup_app().await;
    let token = generate_test_token();

    // 1. Shorten
    let payload = ShortenUrlRequest {
        original_url: "https://github.com".to_string(),
        expires_at: None,
    };
    let res = server
        .post("/api/urls")
        .add_cookie(cookie::Cookie::new("access_token", token))
        .json(&payload)
        .await;

    let url = res.json::<Url>();

    // 2. Resolve (does not need auth)
    let res_get = server.get(&format!("/{}", url.short_code)).await;

    res_get.assert_status(StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(res_get.header("Location"), "https://github.com");
}

#[tokio::test]
async fn test_api_unauthorized() {
    let server = setup_app().await;

    let payload = ShortenUrlRequest {
        original_url: "https://rust-lang.org".to_string(),
        expires_at: None,
    };

    let res = server.post("/api/urls").json(&payload).await;
    res.assert_status(StatusCode::UNAUTHORIZED);
}
