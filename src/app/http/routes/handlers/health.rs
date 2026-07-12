use axum::{extract::State, http::StatusCode, Json};
use api_response::{ApiMessage, ApiResponse};
use crate::app::state::AppState;
use redis_client::ping;

#[tracing::instrument(skip_all, fields(route = "/health/live"))]
pub async fn live() -> (StatusCode, Json<ApiResponse<ApiMessage>>) {
    (
        StatusCode::OK,
        Json(ApiResponse::new(ApiMessage::new(StatusCode::OK.to_string())))
    )
}

#[tracing::instrument(skip_all, fields(route = "/health/live"))]
pub async fn redis(State(state): State<AppState>) -> (StatusCode, Json<ApiResponse<ApiMessage>>) {
    match ping(&state.redis).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::new(ApiMessage::new(StatusCode::OK.to_string())))
        ),
        Err(error) => {
            tracing::warn!(%error, "redis ping failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::new(ApiMessage::new("Redis is unavailable".into()))),
            )
        }
    }
}
