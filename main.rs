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
use db::{postgres::init_pg_pool, redis::init_redis};
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
    let redis = init_redis(&settings.redis_url)
        .await
        .expect("failed to connect to redis");

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let app_state = Arc::new(AppState::new(tx.clone(), redis));

    // Consumer task — two flush triggers:
    //   1. Size  : every 17 accumulated click events
    //   2. Timer : every 30 seconds (so low-traffic links are never stuck)
    let consumer_db = db.clone();
    let consumer_handler = tokio::spawn(async move {
        let mut batch: HashMap<String, u64> = HashMap::new();
        let mut total_clicks: u64 = 0;

        let mut flush_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        // Consume the immediate first tick so the timer starts 30 s from now,
        // not from the moment the task spawns.
        flush_interval.tick().await;

        loop {
            tokio::select! {
                // ── Arm 1: new click event from the channel ──────────────────
                maybe_code = rx.recv() => {
                    match maybe_code {
                        Some(short_code) => {
                            *batch.entry(short_code).or_insert(0) += 1;
                            total_clicks += 1;

                            if total_clicks >= 17 {
                                match url_repository::increment_click_counts(&consumer_db, &batch).await {
                                    Ok(()) => tracing::info!(total_clicks, "click batch flushed (size trigger)"),
                                    Err(e) => tracing::error!("click batch flush failed: {e}"),
                                }
                                batch.clear();
                                total_clicks = 0;
                            }
                        }
                        None => {
                            // Channel closed (server shutting down) — drain remainder
                            if !batch.is_empty() {
                                match url_repository::increment_click_counts(&consumer_db, &batch).await {
                                    Ok(()) => tracing::info!(total_clicks, "click batch flushed (shutdown drain)"),
                                    Err(e) => tracing::error!("click batch final flush failed: {e}"),
                                }
                            }
                            break;
                        }
                    }
                }

                // ── Arm 2: periodic 30-second flush ──────────────────────────
                _ = flush_interval.tick() => {
                    if !batch.is_empty() {
                        match url_repository::increment_click_counts(&consumer_db, &batch).await {
                            Ok(()) => tracing::info!(total_clicks, "click batch flushed (interval trigger)"),
                            Err(e) => tracing::error!("click batch interval flush failed: {e}"),
                        }
                        batch.clear();
                        total_clicks = 0;
                    }
                }
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
