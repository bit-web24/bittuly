mod config;
mod db;
mod handlers;
mod middlewares;
mod models;
mod repository;
mod routes;
mod services;
mod utils;

use config::settings::Settings;
use db::postgres::init_pg_pool;
use routes::router::create_router;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("ApplicationError: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("tower_http=debug,bittuly=info")),
        )
        .init();

    let settings = Settings::from_env()?;
    let db = init_pg_pool(&settings.database_url).await?;
    let app = create_router(db, &settings.mode);
    let listener = TcpListener::bind(&settings.server_addr).await?;

    println!("listening on {} [mode={}]", settings.server_addr, settings.mode);

    axum::serve(listener, app).await?;

    Ok(())
}
