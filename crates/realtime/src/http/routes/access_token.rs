use crate::requests::AccessTokenRequest;
use crate::responses::AccessTokenResponse;
use crate::{ApplicationId, Realtime};
use api_response::{ApiError, ApiResponse};
use auth::TokenCapability;
use axum::Json;
use axum::extract::{Path, State};
use std::sync::Arc;

pub async fn access_token(
    Path(application_id): Path<String>,
    State(realtime): State<Arc<Realtime>>,
    Json(payload): Json<AccessTokenRequest>,
) -> Result<ApiResponse<AccessTokenResponse>, ApiError> {
    let ttl_seconds = 60;

    let application_id = ApplicationId::new(application_id);
    let client_id = payload.client_id;

    // temporary capability
    let capability = r#"{"*": ["publish", "subscribe", "presence"]}"#
        .parse::<TokenCapability>()
        .expect("static realtime capability must be valid");

    let jwt = realtime
        .issue_access_token(
            &application_id,
            client_id.clone(),
            &capability,
            ttl_seconds,
        )
        .map_err(|_| ApiError::unauthorized("Unauthorized"))?;

    let app_id = application_id.as_str();

    let response = AccessTokenResponse {
        application_id: app_id.to_string(),
        client_id,
        jwt,
        ttl_seconds,
    };

    Ok(ApiResponse::new(response))
}
