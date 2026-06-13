use crate::{db::postgres::DbPool, models::Url, repository::url_repository};
use uuid::Uuid;

pub async fn shorten_url(
    db: &DbPool,
    original_url: &str,
    user_id: Uuid,
) -> Result<Option<Url>, sqlx::Error> {
    url_repository::add_shorten_url(db, original_url, user_id).await
}

pub async fn get_original_url(
    db: &DbPool,
    short_code: &str,
) -> Result<Option<String>, sqlx::Error> {
    url_repository::get_original_url(db, short_code).await
}

pub async fn get_urls_page(
    db: &DbPool,
    user_id: Uuid,
    cursor: Option<i64>,
    limit: i64,
    search: Option<String>,
) -> Result<url_repository::UrlsPage, sqlx::Error> {
    url_repository::get_urls_page(db, user_id, cursor, limit, search).await
}

/// Returns `Some(short_code)` if deleted, `None` if not found or not owned by the user.
pub async fn delete_url(
    db: &DbPool,
    url_id: i64,
    user_id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    url_repository::delete_url(db, url_id, user_id).await
}
