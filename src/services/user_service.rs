use crate::db::postgres::DbPool;
use crate::models::user::{CreateUserPayload, UpdateUserPayload, User};
use crate::repository::user_repository;
use uuid::Uuid;

pub async fn create_user(db: &DbPool, payload: CreateUserPayload) -> Result<User, sqlx::Error> {
    user_repository::create_user(db, payload).await
}

pub async fn get_user_by_id(db: &DbPool, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
    user_repository::get_user_by_id(db, user_id).await
}

pub async fn update_user(
    db: &DbPool,
    user_id: Uuid,
    payload: UpdateUserPayload,
) -> Result<User, sqlx::Error> {
    user_repository::update_user(db, user_id, payload).await
}

pub async fn delete_user(db: &DbPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    user_repository::delete_user(db, user_id).await
}
