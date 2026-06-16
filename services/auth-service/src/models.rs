use std::sync::Arc;

use chrono::DateTime;
use chrono::Utc;
use metrics_exporter_prometheus::PrometheusHandle;
use serde::Deserialize;
use serde::Serialize;
use sqlx::PgPool;
use tokio::time::Instant;
use uuid::Uuid;
use validator::Validate;

pub struct AuthState {
    pub db: PgPool,
    pub rabbitmq: shared::deadpool_lapin::Pool,
    #[allow(dead_code)]
    pub started_at: Instant,
    #[allow(dead_code)]
    pub prometheus_handle: PrometheusHandle,
}

impl AuthState {
    pub fn new(
        db: PgPool,
        rabbitmq: shared::deadpool_lapin::Pool,
        prometheus_handle: PrometheusHandle,
    ) -> Self {
        Self {
            db,
            rabbitmq,
            started_at: Instant::now(),
            prometheus_handle,
        }
    }
}

pub type AuthStateRef = Arc<AuthState>;

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
