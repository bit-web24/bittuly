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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let settings = Settings::from_env()?;
    let db = init_pg_pool(&settings.database_url).await?;
    let app = create_router(db);
    let listener = TcpListener::bind(&settings.server_addr).await?;

    println!("listening on {}", settings.server_addr);

    axum::serve(listener, app).await?;

    Ok(())
}
