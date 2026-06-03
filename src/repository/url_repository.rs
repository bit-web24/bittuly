use crate::{db::postgres::DbPool, models::Url};
use uuid::Uuid;

pub async fn add_shorten_url(
    db: &DbPool,
    original_url: &str,
    user_id: Uuid,
) -> Result<Url, sqlx::Error> {
    let mut tx = db.begin().await?;

    let url_id: i64 = sqlx::query_scalar(
        "INSERT INTO urls (original_url, user_id) VALUES ($1, $2) RETURNING url_id",
    )
    .bind(original_url)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    let short_code = base62::encode(url_id as u128);

    let url = sqlx::query_as(
        "UPDATE urls SET short_code = $1 WHERE url_id = $2 RETURNING url_id, short_code, original_url, user_id, created_at, updated_at",
    )
        .bind(&short_code)
        .bind(url_id)
        .fetch_one(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(url)
}

pub async fn get_original_url(
    db: &DbPool,
    short_code: &str,
) -> Result<Option<String>, sqlx::Error> {
    let original_url = sqlx::query_scalar("SELECT original_url FROM urls WHERE short_code = $1")
        .bind(short_code)
        .fetch_optional(db)
        .await?;

    Ok(original_url)
}
pub async fn get_all_urls(db: &DbPool) -> Result<Vec<Url>, sqlx::Error> {
    let urls = sqlx::query_as(
        "SELECT url_id, short_code, original_url, user_id, created_at, updated_at FROM urls",
    )
    .fetch_all(db)
    .await?;

    Ok(urls)
}
