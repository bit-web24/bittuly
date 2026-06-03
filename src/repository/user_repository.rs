use crate::db::postgres::DbPool;
use crate::models::user::{CreateUserPayload, UpdateUserPayload, User};
use uuid::Uuid;

pub async fn create_new_user(db: &DbPool, payload: CreateUserPayload) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO users (user_id, username, email, password) VALUES ($1, $2, $3, $4)")
        .bind(Uuid::new_v4())
        .bind(&payload.username)
        .bind(&payload.email)
        .bind(&payload.password)
        .execute(db)
        .await
        .map_err(|e| e.into())
        .map(|_| ())
}
pub async fn get_user_by_id(db: &DbPool, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as(
        "SELECT user_id AS id, username, email, password, created_at, updated_at FROM users WHERE user_id = $1",
    )
        .bind(user_id)
        .fetch_optional(db)
        .await
}

pub async fn update_user(
    db: &DbPool,
    user_id: Uuid,
    payload: UpdateUserPayload,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as(
        r#"
        UPDATE users
        SET
            username = COALESCE($1, username),
            email = COALESCE($2, email),
            password = COALESCE($3, password),
            updated_at = NOW()
        WHERE user_id = $4
        RETURNING user_id AS id, username, email, password, created_at, updated_at
        "#,
    )
    .bind(payload.username)
    .bind(payload.email)
    .bind(payload.password)
    .bind(user_id)
    .fetch_optional(db)
    .await
}

pub async fn delete_user(db: &DbPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users WHERE user_id = $1")
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(())
}
