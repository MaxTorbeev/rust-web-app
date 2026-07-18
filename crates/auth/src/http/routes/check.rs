use std::sync::Arc;
use axum::extract::State;
use axum::http::HeaderMap;
use api_response::{ApiError, ApiResponse};
use crate::{SessionStore};
use crate::http::responses::LoginResponse;

pub async fn check(
  headers: HeaderMap,
  State(sessions): State<Arc<SessionStore>>,
) -> Result<ApiResponse<LoginResponse>, ApiError> {
  let token = headers
    .get("Authorization")
    .and_then(|h| h.to_str().ok())
    .and_then(|h| h.strip_prefix("Bearer "))
    .ok_or_else(|| ApiError::unauthorized("Missing Bearer Header"))?;

  let session = sessions.find(&token)
    .await
    .map_err(|_| ApiError::unauthorized("Invalid token"))?;

  let resource = LoginResponse::new(token.to_string(), session.user);

  Ok(ApiResponse::new(resource))
}