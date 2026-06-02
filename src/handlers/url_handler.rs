use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::postgres::DbPool,
    repository::url_repository::{
        add_shorten_url,
        get_original_url as find_original_url,
    },
};

#[derive(Deserialize)]
pub struct ShortenUrlRequest {
    pub original_url: String,
    pub short_code: Option<String>,
    pub user_id: String,
}

#[derive(Serialize)]
struct ShortenUrlResponse {
    short_code: String,
}

pub async fn shorten_url(
    State(db): State<DbPool>,
    Json(body): Json<ShortenUrlRequest>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&body.user_id) {
        Ok(user_id) => user_id,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid user_id").into_response(),
    };

    let short_code = body
        .short_code
        .unwrap_or_else(|| Uuid::new_v4().to_string()[..8].to_owned());

    match add_shorten_url(&db, &body.original_url, &short_code, user_id).await {
        Ok(short_code) => (StatusCode::CREATED, Json(ShortenUrlResponse { short_code })).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn get_original_url(
    State(db): State<DbPool>,
    Path(short_code): Path<String>,
) -> impl IntoResponse {
    match find_original_url(&db, &short_code).await {
        Ok(Some(original_url)) => Redirect::temporary(&original_url).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
