use crate::db::postgres::DbPool;
use base62;
use uuid::Uuid;

pub async fn add_shorten_url(
    db: &DbPool,
    original_url: &str,
    user_id: Uuid,
) -> Result<String, sqlx::Error> {
    let mut tx = db.begin().await?;

    let url_id: i64 = sqlx::query_scalar(
        "INSERT INTO urls (original_url, user_id) VALUES ($1, $2) RETURNING url_id",
    )
    .bind(original_url)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    let short_code = base62::encode(url_id as u128);

    sqlx::query("UPDATE urls SET short_code = $1 WHERE url_id = $2")
        .bind(&short_code)
        .bind(url_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(short_code)
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
