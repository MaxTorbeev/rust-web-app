use std::sync::Arc;
use axum::extract::{Path, State};
use api_response::{ApiError, ApiResponse};
use auth::{BearerToken, SessionStore, TokenCapability};
use crate::{ApplicationId, Realtime};
use crate::responses::AccessTokenResponse;

pub async fn access_token(
  Path(application_id): Path<String>,
  bearer_token: BearerToken,
  State(session): State<Arc<SessionStore>>,
  State(realtime): State<Arc<Realtime>>,
) -> Result<ApiResponse<AccessTokenResponse>, ApiError> {
  let ttl_seconds = 60;

  let _session_store = session
    .find(bearer_token.as_str())
    .await
    .map_err(|e| {
      ApiError::unauthorized("Invalid session")
    });

  let application_id = ApplicationId::new(application_id);

  // temporary client id
  let client_id = "maxtor".to_string();

  // temporary capability
  let capability = r#"{"*": ["publish", "subscribe", "presence"]}"#
    .parse::<TokenCapability>()
    .expect("static realtime capability must be valid");


  let jwt = realtime.issue_access_token(
    &application_id.clone(),
    client_id.clone(),
    &capability,
    ttl_seconds,
  ).map_err(|_| {
    ApiError::unauthorized("Unauthorized")
  })?;


  let app_id = application_id.as_str();

  let response = AccessTokenResponse {
    application_id: app_id.to_string(),
    client_id,
    jwt,
    ttl_seconds
  };

  Ok(ApiResponse::new(response))
}