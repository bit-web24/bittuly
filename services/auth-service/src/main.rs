mod debug_handler;
mod email;
mod handlers;
mod models;
mod otp_store;
mod repository;
mod routes;
mod service;

use std::sync::Arc;

use shared::config;
use shared::postgres;

#[tokio::main]
async fn main() {
    let settings = config::Settings::from_env().expect("Failed to load setting from environment");
    let db = postgres::init_pg_pool(&settings.database_url)
        .await
        .expect("Failed to connect to Database");
    let auth_state = Arc::new(models::AuthState { db });
    let app = routes::user_routes().with_state(auth_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
