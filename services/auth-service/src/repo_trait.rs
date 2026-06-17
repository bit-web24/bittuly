use crate::models::{CreateUserPayload, UpdateUserPayload, User};
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait UserRepo: Send + Sync {
    async fn create_user(&self, payload: CreateUserPayload) -> Result<User, sqlx::Error>;
    async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<User>, sqlx::Error>;
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error>;
    async fn update_user(
        &self,
        user_id: Uuid,
        payload: UpdateUserPayload,
    ) -> Result<User, sqlx::Error>;
    async fn delete_user(&self, user_id: Uuid) -> Result<(), sqlx::Error>;
    async fn ping(&self) -> Result<(), sqlx::Error>;
}
