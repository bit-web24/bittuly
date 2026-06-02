use crate::{db::postgres::DbPool, repository::url_repository};
use uuid::Uuid;

pub async fn shorten_url(
    db: &DbPool,
    original_url: &str,
    user_id: Uuid,
) -> Result<String, sqlx::Error> {
    url_repository::add_shorten_url(db, original_url, user_id).await
}

pub async fn get_original_url(
    db: &DbPool,
    short_code: &str,
) -> Result<Option<String>, sqlx::Error> {
    url_repository::get_original_url(db, short_code).await
}
