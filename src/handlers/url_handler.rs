use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{db::postgres::DbPool, services::url_service};

#[derive(Deserialize)]
pub struct ShortenUrlRequest {
    pub original_url: String,
    pub user_id: String,
}

#[derive(Deserialize)]
pub struct GetAllUrlsRequest {
    pub user_id: String,
}

pub async fn get_all_urls(
    State(db): State<DbPool>,
    Json(body): Json<GetAllUrlsRequest>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&body.user_id) {
        Ok(user_id) => user_id,
        Err(err) => {
            eprintln!("{:?}", err);
            return (StatusCode::BAD_REQUEST, "invalid user_id").into_response();
        }
    };
    match url_service::get_all_urls(&db, user_id).await {
        Ok(urls) => (StatusCode::OK, Json(urls)).into_response(),
        Err(err) => {
            eprintln!("{:?}", err);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn shorten_url(
    State(db): State<DbPool>,
    Json(body): Json<ShortenUrlRequest>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&body.user_id) {
        Ok(user_id) => user_id,
        Err(err) => {
            eprintln!("{:?}", err);
            return (StatusCode::BAD_REQUEST, "invalid user_id").into_response();
        }
    };

    match url_service::shorten_url(&db, &body.original_url, user_id).await {
        Ok(url) => (StatusCode::CREATED, Json(url)).into_response(),
        Err(err) => {
            eprintln!("{:?}", err);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_original_url(
    State(db): State<DbPool>,
    Path(short_code): Path<String>,
) -> impl IntoResponse {
    match url_service::get_original_url(&db, &short_code).await {
        Ok(Some(original_url)) => Redirect::temporary(&original_url).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(err) => {
            eprintln!("{:?}", err);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
