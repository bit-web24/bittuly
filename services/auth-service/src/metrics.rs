use crate::models::AuthStateRef;
use axum::extract::State;
use axum::response::IntoResponse;

pub async fn metrics(State(state): State<AuthStateRef>) -> impl IntoResponse {
    state.prometheus_handle.render()
}
