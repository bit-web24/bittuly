use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;
use validator::Validate;

#[derive(sqlx::FromRow, Serialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    #[serde(skip)]
    pub password: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct AuthUserResponse {
    pub user: User,
    pub token: String,
    pub refresh_token: String,
}

#[derive(Deserialize, Validate)]
pub struct CreateUserPayload {
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 6))]
    pub password: String,
}

#[derive(Deserialize, Validate)]
pub struct UpdateUserPayload {
    #[validate(length(min = 3, max = 50))]
    pub username: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    #[validate(length(min = 6))]
    pub password: Option<String>,
}

#[derive(Deserialize, Validate)]
pub struct LoginPayload {
    #[validate(email)]
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Validate)]
pub struct VerifyOtpPayload {
    /// The short-lived pending JWT issued by POST /users/signup.
    pub pending_token: String,
    /// The 6-digit OTP the user received by email.
    #[validate(length(min = 6, max = 6))]
    pub otp: String,
}
