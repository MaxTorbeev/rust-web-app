use std::sync::Arc;
use axum::extract::State;
use axum::http::HeaderMap;
use api_response::{ApiError, ApiMessage, ApiResponse};
use crate::{SessionStore};

pub async fn check(
  headers: HeaderMap,
  State(sessions): State<Arc<SessionStore>>,
) -> Result<ApiResponse<ApiMessage>, ApiError> {

  let token = headers
    .get("Authorization")
    .and_then(|h| h.to_str().ok())
    .and_then(|h| h.strip_prefix("Bearer: "))
    .ok_or_else(|| ApiError::unauthorized("Missing Bearer Header"))?;

  let token = sessions.find(&token).await.unwrap();

  Ok(ApiResponse::new(ApiMessage::new(token.to_string())))
}