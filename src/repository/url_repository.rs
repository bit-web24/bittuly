use crate::db::postgres::DbPool;
use uuid::Uuid;

pub async fn add_shorten_url(
    db: &DbPool,
    original_url: &str,
    short_code: &str,
    user_id: Uuid,
) -> Result<String, sqlx::Error> {
    let short_code = sqlx::query_scalar(
        "INSERT INTO urls (original_url, short_code, user_id) VALUES ($1, $2, $3) RETURNING short_code",
    )
        .bind(original_url)
        .bind(short_code)
        .bind(user_id)
        .fetch_one(db)
        .await?;

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
