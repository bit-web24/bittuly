use crate::models::Url;
use crate::repo_trait::UrlRepo;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared::postgres::DbPool;
use uuid::Uuid;

/// One page of URLs for a user.
pub struct UrlsPage {
    pub urls: Vec<Url>,
    /// Hex-encoded `url_id` of the last item; `None` means no further pages.
    pub next_cursor: Option<String>,
}

pub struct PgUrlRepo(pub DbPool);

#[async_trait]
impl UrlRepo for PgUrlRepo {
    async fn add_shorten_url(
        &self,
        original_url: &str,
        user_id: Uuid,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<Option<Url>, sqlx::Error> {
        let db = &self.0;
        let existing_url: Option<Url> = sqlx::query_as(
            "SELECT url_id, short_code, original_url, user_id, click_count, expires_at, created_at, updated_at
             FROM urls WHERE original_url = $1 AND user_id = $2"
        )
        .bind(original_url)
        .bind(user_id)
        .fetch_optional(db)
        .await?;

        if let Some(existing) = existing_url {
            if let Some(exp) = existing.expires_at {
                if exp < Utc::now() {
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
            }
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

    async fn get_original_url(
        &self,
        short_code: &str,
    ) -> Result<Option<(String, Option<DateTime<Utc>>)>, sqlx::Error> {
        let row = sqlx::query_as::<_, (String, Option<DateTime<Utc>>)>(
            "SELECT original_url, expires_at FROM urls WHERE short_code = $1",
        )
        .bind(short_code)
        .fetch_optional(&self.0)
        .await?;

        Ok(row)
    }

    async fn get_urls_page(
        &self,
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
        .bind(limit + 1)
        .bind(search_pattern)
        .fetch_all(&self.0)
        .await?;

        let has_next = rows.len() as i64 > limit;
        let mut urls = rows;
        if has_next {
            urls.pop();
        }

        let next_cursor = if has_next {
            urls.last().map(|u| format!("{:x}", u.url_id))
        } else {
            None
        };

        Ok(UrlsPage { urls, next_cursor })
    }

    async fn delete_url(&self, url_id: i64, user_id: Uuid) -> Result<Option<String>, sqlx::Error> {
        let row: Option<(String,)> = sqlx::query_as(
            "DELETE FROM urls WHERE url_id = $1 AND user_id = $2 RETURNING short_code",
        )
        .bind(url_id)
        .bind(user_id)
        .fetch_optional(&self.0)
        .await?;

        Ok(row.map(|(short_code,)| short_code))
    }

    async fn ping(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1").execute(&self.0).await?;
        Ok(())
    }
}
