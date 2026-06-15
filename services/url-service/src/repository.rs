use crate::models::Url;
use shared::postgres::DbPool;
use uuid::Uuid;

/// Creates a shortened URL.
/// Returns `Ok(Some(url))` on success.
/// Returns `Ok(None)` if this `original_url` was already shortened by `user_id`
/// (avoids burning a BIGSERIAL url_id for a duplicate request).
pub async fn add_shorten_url(
    db: &DbPool,
    original_url: &str,
    user_id: Uuid,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<Option<Url>, sqlx::Error> {
    // Pre-check: see if the user already shortened this URL
    let existing_url: Option<Url> = sqlx::query_as(
        "SELECT url_id, short_code, original_url, user_id, click_count, expires_at, created_at, updated_at
         FROM urls WHERE original_url = $1 AND user_id = $2"
    )
    .bind(original_url)
    .bind(user_id)
    .fetch_optional(db)
    .await?;

    if let Some(existing) = existing_url {
        // If it exists, check if it's currently EXPIRED
        if let Some(exp) = existing.expires_at
            && exp < chrono::Utc::now()
        {
            // It's expired! Reactivate it by updating `expires_at`
            let reactivated: Url = sqlx::query_as(
                    "UPDATE urls SET expires_at = $1, updated_at = now()
                     WHERE url_id = $2
                     RETURNING url_id, short_code, original_url, user_id, click_count, expires_at, created_at, updated_at"
                )
                .bind(expires_at)
                .bind(existing.url_id)
                .fetch_one(db)
                .await?;

            return Ok(Some(reactivated));
        }

        // It exists and is NOT expired -> Return Conflict
        return Ok(None);
    }

    let mut tx = db.begin().await?;

    let url_id: i64 = match sqlx::query_scalar(
        "INSERT INTO urls (original_url, user_id, expires_at) VALUES ($1, $2, $3) RETURNING url_id",
    )
    .bind(original_url)
    .bind(user_id)
    .bind(expires_at)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(id) => id,
        // Race condition: another request snuck in between our SELECT and INSERT
        Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23505") => {
            tx.rollback().await.ok();
            return Ok(None);
        }
        Err(e) => return Err(e),
    };

    let short_code = base62::encode(url_id as u128);

    let url = sqlx::query_as(
        "UPDATE urls SET short_code = $1 WHERE url_id = $2 \
         RETURNING url_id, short_code, original_url, user_id, click_count, expires_at, created_at, updated_at",
    )
    .bind(&short_code)
    .bind(url_id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Some(url))
}

pub async fn get_original_url(
    db: &DbPool,
    short_code: &str,
) -> Result<Option<(String, Option<chrono::DateTime<chrono::Utc>>)>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT original_url, expires_at FROM urls WHERE short_code = $1",
    )
    .bind(short_code)
    .fetch_optional(db)
    .await?;

    Ok(row)
}

/// One page of URLs for a user.
pub struct UrlsPage {
    pub urls: Vec<Url>,
    /// Hex-encoded `url_id` of the last item; `None` means no further pages.
    pub next_cursor: Option<String>,
}

/// Fetches one page of a user's URLs ordered by `url_id DESC` (newest first).
///
/// `cursor`  — opaque value returned by the previous call; pass `None` for the
///             first page.
/// `limit`   — page size (clamped to 1–100 internally).
///
/// Internally fetches `limit + 1` rows so it can determine whether a next page
/// exists without a separate COUNT query.
pub async fn get_urls_page(
    db: &DbPool,
    user_id: Uuid,
    cursor: Option<i64>,
    limit: i64,
    search: Option<String>,
) -> Result<UrlsPage, sqlx::Error> {
    let limit = limit.clamp(1, 100);

    let search_pattern = search.map(|s| format!("%{}%", s));

    let rows: Vec<Url> = sqlx::query_as(
        "SELECT url_id, short_code, original_url, user_id, click_count, expires_at, created_at, updated_at
         FROM urls
         WHERE user_id = $1
           AND ($2::bigint IS NULL OR url_id < $2)
           AND ($4::text IS NULL OR original_url ILIKE $4 OR short_code ILIKE $4)
         ORDER BY url_id DESC
         LIMIT $3",
    )
    .bind(user_id)
    .bind(cursor)
    .bind(limit + 1) // fetch one extra to detect next page
    .bind(search_pattern)
    .fetch_all(db)
    .await?;

    let has_next = rows.len() as i64 > limit;
    let mut urls = rows;
    if has_next {
        urls.pop(); // discard the sentinel item
    }

    let next_cursor = if has_next {
        // encode as hex so the internal int is opaque to API consumers
        urls.last().map(|u| format!("{:x}", u.url_id))
    } else {
        None
    };

    Ok(UrlsPage { urls, next_cursor })
}

/// Deletes a URL by its numeric id, scoped to the owning user.
/// Returns `Some(short_code)` if deleted, `None` if not found or not owned.
pub async fn delete_url(
    db: &DbPool,
    url_id: i64,
    user_id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> =
        sqlx::query_as("DELETE FROM urls WHERE url_id = $1 AND user_id = $2 RETURNING short_code")
            .bind(url_id)
            .bind(user_id)
            .fetch_optional(db)
            .await?;

    Ok(row.map(|(short_code,)| short_code))
}
