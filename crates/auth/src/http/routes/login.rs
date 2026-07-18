use crate::{AuthConfig, Session, SessionStore, UserIdentity};
use crate::http::requests::LoginRequest;
use axum::{Json, extract::State};
use std::sync::Arc;
use api_response::{ApiError, ApiMessage, ApiResponse};
use crate::authenticator::Authenticator;
use crate::http::responses::LoginResponse;

#[tracing::instrument(skip_all, fields(route = "/auth/login"))]
pub async fn login(
  State(auth): State<Arc<AuthConfig>>,
  State(sessions): State<Arc<SessionStore>>,
  Json(payload): Json<LoginRequest>
) -> Result<ApiResponse<LoginResponse>, ApiError> {

  let is_verify = Authenticator::is_verify(
    auth.as_ref(),
    payload.login.as_str(),
    payload.password.as_str()
  );

  if !is_verify {
    return Err(ApiError::unauthorized("Invalid credentials"))
  }

  let session = Session {
    user: UserIdentity::new(payload.login)
  };

  let token = sessions.create(session).await.unwrap();

  let session = sessions.find(&token).await.unwrap();

  Ok(ApiResponse::new(LoginResponse::new(token, session.user)))
}
