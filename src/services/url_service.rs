use crate::{db::postgres::DbPool, models::Url, repository::url_repository};
use uuid::Uuid;

pub async fn shorten_url(
    db: &DbPool,
    original_url: &str,
    user_id: Uuid,
) -> Result<Url, sqlx::Error> {
    url_repository::add_shorten_url(db, original_url, user_id).await
}

pub async fn get_original_url(
    db: &DbPool,
    short_code: &str,
) -> Result<Option<String>, sqlx::Error> {
    url_repository::get_original_url(db, short_code).await
}

pub async fn get_all_urls(db: &DbPool, user_id: Uuid) -> Result<Vec<Url>, sqlx::Error> {
    url_repository::get_all_urls(db, user_id).await
}

/// Returns `Some(short_code)` if deleted, `None` if not found or not owned by the user.
pub async fn delete_url(
    db: &DbPool,
    url_id: i64,
    user_id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    url_repository::delete_url(db, url_id, user_id).await
}
