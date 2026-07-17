use crate::AuthConfig;
use crate::http::requests::LoginRequest;
use axum::http::StatusCode;
use axum::{Json, extract::State};
use std::sync::Arc;
use api_response::{ApiMessage, ApiResponse};
use crate::authenticator::Authenticator;

#[tracing::instrument(skip_all, fields(route = "/auth/login"))]
pub async fn login(State(auth): State<Arc<AuthConfig>>, Json(payload): Json<LoginRequest>) -> (StatusCode, Json<ApiResponse<ApiMessage>>) {

  let is_verify = Authenticator::is_verify(
    auth.as_ref(),
    payload.login.as_str(),
    payload.password.as_str()
  );

  if is_verify {
    return (
      StatusCode::OK,
      Json(ApiResponse::new(ApiMessage::new(StatusCode::OK.to_string())))
    )
  }

  (
    StatusCode::FORBIDDEN,
    Json(ApiResponse::new(ApiMessage::new(StatusCode::FORBIDDEN.to_string())))
  )
}
