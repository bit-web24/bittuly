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
use routes::router::create_router;
use tokio::{net::TcpListener, sync::mpsc};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let app_state = Arc::new(AppState::from(tx.clone()));

    let consumer_handler = tokio::spawn(async move {
        let mut batch: HashMap<String, u64> = HashMap::new();
        while let Some(res) = rx.recv().await {
            *batch.entry(res).or_insert(0) += 1;
        }
    });

    if let Err(err) = run(app_state).await {
        eprintln!("ApplicationError: {err}");
        drop(tx);
        consumer_handler.await.unwrap();
        std::process::exit(1);
    }
}

async fn run(state: Arc<AppState>) -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("tower_http=debug,bittuly=info")),
        )
        .init();

    let settings = Settings::from_env()?;
    let db = init_pg_pool(&settings.database_url).await?;
    let app = create_router(db, &settings.mode, &settings.cors_origin, state);
    let listener = TcpListener::bind(&settings.server_addr).await?;

    println!(
        "listening on {} [mode={} cors={}]",
        settings.server_addr, settings.mode, settings.cors_origin
    );

    axum::serve(listener, app).await?;

    Ok(())
}
