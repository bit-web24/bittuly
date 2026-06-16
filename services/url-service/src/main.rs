mod handlers;
mod health;
mod metrics;
mod models;
mod repository;
mod routes;
mod services;

use metrics_exporter_prometheus::PrometheusBuilder;
use shared::config;
use shared::postgres;
use shared::redis;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tower_http=debug,url_service=debug,shared=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let settings = config::UrlConfig::from_env().expect("Failed to load setting from environment");
    let db = postgres::init_pg_pool(&settings.database_url)
        .await
        .expect("Failed to connect to Database");
    let redis = redis::init_redis(&settings.redis_url)
        .await
        .expect("Failed to connect to Redis");
    let amqp_url = std::env::var("RABBITMQ_URL").expect("RABBITMQ_URL must be set");
    let rabbitmq = shared::rabbitmq::init_rabbitmq_pool(&amqp_url).await;

    // Consumer logic moved to consumer-service (Phase 2 RabbitMQ)
    let prometheus_handle = PrometheusBuilder::new()
        .install_recorder()
        .expect("Failed to install Prometheus recorder");

    let url_state = Arc::new(models::UrlState::new(
        rabbitmq,
        redis,
        db,
        settings.cors_origin.clone(),
        prometheus_handle,
    ));

    let cors = CorsLayer::new()
        .allow_origin(
            settings
                .cors_origin
                .parse::<axum::http::HeaderValue>()
                .expect("Invalid CORS origin"),
        )
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ])
        .allow_credentials(true);

    let app = routes::url_routes()
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(url_state);

    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", settings.server_addr, settings.server_port))
            .await
            .expect("failed to bind listener");

    println!(
        "Url service listening on {}:{} [mode={} cors={}]",
        settings.server_addr, settings.server_port, settings.mode, settings.cors_origin
    );

    if let Err(err) = axum::serve(listener, app).await {
        eprintln!("server error: {err}");
    }
}
