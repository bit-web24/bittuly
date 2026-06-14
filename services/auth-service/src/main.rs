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

#[tokio::main]
async fn main() {
    let settings = config::AuthConfig::from_env().expect("Failed to load setting from environment");
    let db = postgres::init_pg_pool(&settings.database_url)
        .await
        .expect("Failed to connect to Database");

    let auth_state = Arc::new(models::AuthState::new(db));
    let app = axum::Router::new()
        .nest("/api/auth", routes::auth_routes())
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
