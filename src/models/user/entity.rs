use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

struct User {
    id: Uuid,
    username: String,
    email: String,
    password: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
