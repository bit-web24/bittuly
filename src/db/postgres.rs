use sqlx::{postgres::PgPoolOptions, PgPool};

pub type DbPool = PgPool;

pub async fn init_pg_pool(database_url: &str) -> Result<DbPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
}
