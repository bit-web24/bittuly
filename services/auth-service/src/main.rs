mod debug_handler;
mod email;
mod handlers;
mod health;
mod models;
mod otp_store;
mod repository;
mod routes;
mod services;

use shared::config;
use shared::postgres;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tower_http=debug,auth_service=debug,shared=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let settings = config::AuthConfig::from_env().expect("Failed to load setting from environment");
    let db = postgres::init_pg_pool(&settings.database_url)
        .await
        .expect("Failed to connect to Database");

    let auth_state = Arc::new(models::AuthState::new(db));
    
    let cors = CorsLayer::new()
        .allow_origin(
            settings
                .cors_origin
                .parse::<axum::http::HeaderValue>()
                .expect("Invalid CORS origin"),
        )
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_credentials(true);

    let app = axum::Router::new()
        .nest("/api/auth", routes::auth_routes())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(auth_state);

    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", settings.server_addr, settings.server_port))
            .await
            .expect("failed to bind listener");

    println!(
        "Auth service listening on {} [mode={} cors={}]",
        settings.server_addr, settings.mode, settings.cors_origin
    );

    if let Err(err) = axum::serve(listener, app).await {
        eprintln!("Auth server error: {err}");
    }
}
