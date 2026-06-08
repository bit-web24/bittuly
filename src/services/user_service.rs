use crate::db::postgres::DbPool;
use crate::middlewares::jwt::{create_access_token, create_refresh_token};
use crate::models::user::{AuthUserResponse, CreateUserPayload, UpdateUserPayload, User};
use crate::repository::user_repository;
use uuid::Uuid;

type ServiceResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub async fn create_user(
    db: &DbPool,
    mut payload: CreateUserPayload,
) -> ServiceResult<AuthUserResponse> {
    payload.password = bcrypt::hash(&payload.password, bcrypt::DEFAULT_COST)?;
    let user = user_repository::create_user(db, payload).await?;
    let token = create_access_token(user.id)?;
    let refresh_token = create_refresh_token(user.id)?;

    Ok(AuthUserResponse {
        user,
        token,
        refresh_token,
    })
}

pub async fn login(db: &DbPool, email: &str, password: &str) -> ServiceResult<AuthUserResponse> {
    let user: User = user_repository::get_user_by_email(db, email)
        .await?
        .ok_or("invalid credentials")?;

    if !bcrypt::verify(password, &user.password)? {
        return Err("invalid credentials".into());
    }

    let token = create_access_token(user.id)?;
    let refresh_token = create_refresh_token(user.id)?;

    Ok(AuthUserResponse {
        user,
        token,
        refresh_token,
    })
}

pub async fn get_user_by_id(db: &DbPool, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
    user_repository::get_user_by_id(db, user_id).await
}

pub async fn update_user(
    db: &DbPool,
    user_id: Uuid,
    mut payload: UpdateUserPayload,
) -> ServiceResult<AuthUserResponse> {
    if let Some(ref plain) = payload.password {
        payload.password = Some(bcrypt::hash(plain, bcrypt::DEFAULT_COST)?);
    }
    let user = user_repository::update_user(db, user_id, payload).await?;
    let token = create_access_token(user.id)?;
    let refresh_token = create_refresh_token(user.id)?;

    Ok(AuthUserResponse {
        user,
        token,
        refresh_token,
    })
}

pub async fn delete_user(db: &DbPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    user_repository::delete_user(db, user_id).await
}
