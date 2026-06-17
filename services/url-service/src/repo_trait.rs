use crate::models::Url;
use crate::repository::UrlsPage;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[async_trait]
pub trait UrlRepo: Send + Sync {
    async fn add_shorten_url(
        &self,
        original_url: &str,
        user_id: Uuid,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<Option<Url>, sqlx::Error>;

    async fn get_original_url(
        &self,
        short_code: &str,
    ) -> Result<Option<(String, Option<DateTime<Utc>>)>, sqlx::Error>;

    async fn get_urls_page(
        &self,
        user_id: Uuid,
        cursor: Option<i64>,
        limit: i64,
        search: Option<String>,
    ) -> Result<UrlsPage, sqlx::Error>;

    async fn delete_url(&self, url_id: i64, user_id: Uuid) -> Result<Option<String>, sqlx::Error>;

    async fn ping(&self) -> Result<(), sqlx::Error>;
}
