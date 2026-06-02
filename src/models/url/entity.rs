use chrono::DateTime;
use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

#[derive(sqlx::FromRow, Serialize)]
pub struct Url {
    pub url_id: i64,
    pub short_code: String,
    pub original_url: String,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
