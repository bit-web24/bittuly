use crate::models::{CreateUserPayload, UpdateUserPayload, User};
use shared::postgres::DbPool;
use uuid::Uuid;

pub async fn create_user(db: &DbPool, payload: CreateUserPayload) -> Result<User, sqlx::Error> {
    sqlx::query_as(
        r#"
        INSERT INTO users (id, username, email, password)
        VALUES ($1, $2, $3, $4)
        RETURNING id, username, email, password, created_at, updated_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&payload.username)
    .bind(&payload.email)
    .bind(&payload.password)
    .fetch_one(db)
    .await
}

pub async fn get_user_by_id(db: &DbPool, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, username, email, password, created_at, updated_at FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
}

pub async fn get_user_by_email(db: &DbPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, username, email, password, created_at, updated_at FROM users WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(db)
    .await
}

pub async fn update_user(
    db: &DbPool,
    user_id: Uuid,
    payload: UpdateUserPayload,
) -> Result<User, sqlx::Error> {
    sqlx::query_as(
        r#"
        UPDATE users
        SET
            username = COALESCE($1, username),
            email = COALESCE($2, email),
            password = COALESCE($3, password),
            updated_at = NOW()
        WHERE id = $4
        RETURNING id, username, email, password, created_at, updated_at
        "#,
    )
    .bind(payload.username)
    .bind(payload.email)
    .bind(payload.password)
    .bind(user_id)
    .fetch_one(db)
    .await
}

pub async fn delete_user(db: &DbPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(())
}
