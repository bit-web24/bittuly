use axum::{
    extract::{Extension, Json, Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use validator::Validate;

use crate::{db::postgres::DbPool, middlewares::jwt::Claims, services::url_service};

#[derive(Deserialize, Validate)]
pub struct ShortenUrlRequest {
    #[validate(url)]
    pub original_url: String,
}

pub async fn get_all_urls(
    State(db): State<DbPool>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    match url_service::get_all_urls(&db, claims.sub).await {
        Ok(urls) => (StatusCode::OK, Json(urls)).into_response(),
        Err(err) => {
            tracing::error!("{:?}", err);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn shorten_url(
    State(db): State<DbPool>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<ShortenUrlRequest>,
) -> impl IntoResponse {
    if let Err(errors) = body.validate() {
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(errors.to_string())).into_response();
    }
    match url_service::shorten_url(&db, &body.original_url, claims.sub).await {
        Ok(url) => (StatusCode::CREATED, Json(url)).into_response(),
        Err(err) => {
            tracing::error!("{:?}", err);
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
            tracing::error!("{:?}", err);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
