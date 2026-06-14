mod consumer;
mod handlers;
mod health;
mod models;
mod repository;
mod routes;
mod services;

use shared::config;
use shared::postgres;
use shared::redis;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use tokio::sync::mpsc;

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

    // Consumer task — two flush triggers:
    //   1. Size  : every 17 accumulated click events
    //   2. Timer : every 30 seconds (so low-traffic links are never stuck)
    let consumer_db = db.clone();
    let (tx, rx) = mpsc::unbounded_channel::<String>();
    let consumer_handler = consumer::spawn_consumer(rx, consumer_db);
    let url_state = Arc::new(models::UrlState::new(
        tx.clone(),
        redis,
        db,
        settings.cors_origin.clone(),
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
        .allow_headers([axum::http::header::CONTENT_TYPE])
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

    // Signal consumer to stop and wait for it to drain
    drop(tx);
    consumer_handler.await.unwrap();
}
