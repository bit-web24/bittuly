use crate::db::postgres::DbPool;
use crate::models::user::{CreateUserPayload, UpdateUserPayload};
use crate::services::user_service;
use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use uuid::Uuid;

pub async fn create_user(
    State(db): State<DbPool>,
    Json(payload): Json<CreateUserPayload>,
) -> impl IntoResponse {
    match user_service::create_user(&db, payload).await {
        Ok(user) => (StatusCode::CREATED, Json(user)).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(err.to_string())).into_response(),
    }
}

pub async fn get_user_by_id(
    State(db): State<DbPool>,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&user_id) {
        Ok(user_id) => user_id,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid user_id").into_response(),
    };

    match user_service::get_user_by_id(&db, user_id).await {
        Ok(Some(user)) => (StatusCode::OK, Json(user)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(err.to_string())).into_response(),
    }
}

pub async fn update_user(
    State(db): State<DbPool>,
    Path(user_id): Path<String>,
    Json(payload): Json<UpdateUserPayload>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&user_id) {
        Ok(user_id) => user_id,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid user_id").into_response(),
    };

    match user_service::update_user(&db, user_id, payload).await {
        Ok(user) => (StatusCode::OK, Json(user)).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(err.to_string())).into_response(),
    }
}

pub async fn delete_user(
    State(db): State<DbPool>,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&user_id) {
        Ok(user_id) => user_id,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid user_id").into_response(),
    };

    match user_service::delete_user(&db, user_id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(err.to_string())).into_response(),
    }
}
