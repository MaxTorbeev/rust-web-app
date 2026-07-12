use crate::AuthConfig;
use crate::http::requests::LoginRequest;
use axum::http::StatusCode;
use axum::{Json, extract::State};
use std::sync::Arc;
use api_response::{ApiMessage, ApiResponse};

#[tracing::instrument(skip_all, fields(route = "/auth/login"))]
pub async fn login(
  State(auth): State<Arc<AuthConfig>>,
  Json(payload): Json<LoginRequest>,
) -> (StatusCode, Json<ApiResponse<ApiMessage>>) {
  (
    StatusCode::OK,
    Json(ApiResponse::new(ApiMessage::new(StatusCode::OK.to_string())))
  )
}
