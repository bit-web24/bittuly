use std::collections::HashMap;

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
        "UPDATE urls SET short_code = $1 WHERE url_id = $2 \
         RETURNING url_id, short_code, original_url, user_id, click_count, created_at, updated_at",
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

pub async fn get_all_urls(db: &DbPool, user_id: Uuid) -> Result<Vec<Url>, sqlx::Error> {
    let urls = sqlx::query_as(
        "SELECT url_id, short_code, original_url, user_id, click_count, created_at, updated_at \
         FROM urls WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    Ok(urls)
}

/// Deletes a URL by its numeric id, scoped to the owning user.
/// Returns `true` if a row was deleted, `false` if nothing matched.
pub async fn delete_url(db: &DbPool, url_id: i64, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let result =
        sqlx::query("DELETE FROM urls WHERE url_id = $1 AND user_id = $2")
            .bind(url_id)
            .bind(user_id)
            .execute(db)
            .await?;

    Ok(result.rows_affected() > 0)
}

/// Flushes a batch of (short_code → click delta) into the database.
/// Called only by the consumer task — not exposed through the service layer.
/// Uses a single unnest-based UPDATE — one query, one round-trip, no loop.
pub async fn increment_click_counts(
    db: &DbPool,
    batch: &HashMap<String, u64>,
) -> Result<(), sqlx::Error> {
    let (codes, deltas): (Vec<String>, Vec<i64>) = batch
        .iter()
        .map(|(code, &count)| (code.clone(), count as i64))
        .unzip();

    sqlx::query(
        "UPDATE urls \
         SET click_count = click_count + d.delta \
         FROM (SELECT unnest($1::text[]) AS code, \
                      unnest($2::bigint[]) AS delta) AS d \
         WHERE urls.short_code = d.code",
    )
    .bind(&codes)
    .bind(&deltas)
    .execute(db)
    .await?;

    Ok(())
}

