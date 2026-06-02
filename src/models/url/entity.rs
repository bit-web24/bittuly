use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

struct Url {
    id: i64,
    short_code: String,
    original_url: String,
    short_code: String,
    user_id: Uuid,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
