use axum::extract::{Json, Path, Query};
use serde::Deserialize;

#[derive(Deserialize)]
struct ShortenUrlRequest {
    original_url: String,
}

async fn shorten_url(Json(body): Json<ShortenUrlRequest>) -> &'static str {
    let ShortenUrlRequest { original_url } = body;
}

async fn get_original_url(Path(short_code): Path<String>) -> &'static str {}
