mod config;
mod db;
mod handlers;
mod models;
mod repository;
mod routes;
mod services;

use std::error::Error;

use config::settings::Settings;
use db::postgres::init_pg_pool;
use routes::router::create_router;
use tokio::net::TcpListener;
use tracing_subscriber;

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("ApplicationError: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let settings = Settings::from_env()?;
    let db = init_pg_pool(&settings.database_url).await?;
    let app = create_router(db);
    let listener = TcpListener::bind(&settings.server_addr).await?;

    println!("listening on {}", settings.server_addr);

    axum::serve(listener, app).await?;

    Ok(())
}
