use crate::models::UrlStateRef;
use axum::extract::State;
use axum::response::IntoResponse;

pub async fn metrics(State(state): State<UrlStateRef>) -> impl IntoResponse {
    state.prometheus_handle.render()
}
