mod app;
mod config;
mod db;
mod handlers;
mod middlewares;
mod models;
mod repository;
mod routes;
mod services;
mod utils;

use std::{collections::HashMap, sync::Arc};

use app::state::AppState;
use config::settings::Settings;
use db::postgres::init_pg_pool;
use repository::url_repository;
use routes::router::create_router;
use tokio::{net::TcpListener, sync::mpsc};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("tower_http=debug,bittuly=info")),
        )
        .init();

    let settings = Settings::from_env().expect("failed to load settings");
    let db = init_pg_pool(&settings.database_url)
        .await
        .expect("failed to connect to database");

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let app_state = Arc::new(AppState::from(tx.clone()));

    // Consumer task: batch click events and flush to DB every 17 unique entries
    let consumer_db = db.clone();
    let consumer_handler = tokio::spawn(async move {
        let mut batch: HashMap<String, u64> = HashMap::new();

        while let Some(short_code) = rx.recv().await {
            *batch.entry(short_code).or_insert(0) += 1;

            if batch.len() >= 17 {
                if let Err(e) = url_repository::increment_click_counts(&consumer_db, &batch).await
                {
                    tracing::error!("click count flush failed: {e}");
                }
                batch.clear();
            }
        }

        // Channel closed — drain whatever is left
        if !batch.is_empty() {
            if let Err(e) = url_repository::increment_click_counts(&consumer_db, &batch).await {
                tracing::error!("click count final flush failed: {e}");
            }
        }
    });

    let app = create_router(db, &settings.mode, &settings.cors_origin, app_state);
    let listener = TcpListener::bind(&settings.server_addr)
        .await
        .expect("failed to bind listener");

    println!(
        "listening on {} [mode={} cors={}]",
        settings.server_addr, settings.mode, settings.cors_origin
    );

    if let Err(err) = axum::serve(listener, app).await {
        eprintln!("server error: {err}");
    }

    // Signal consumer to stop and wait for it to drain
    drop(tx);
    consumer_handler.await.unwrap();
}
