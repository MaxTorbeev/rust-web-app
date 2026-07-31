use std::sync::Arc;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap};
use axum::Json;
use api_response::{ApiError, ApiResponse};
use support::decode_to_string;
use crate::{ApplicationKeyName, Message, ProtocolMessage, Realtime};
use crate::requests::broadcast_message::BroadcastMessage;
use crate::responses::BroadcastMessageResponse;

pub async fn broadcast_message(
  Path(channel): Path<String>,
  headers: HeaderMap,
  State(realtime): State<Arc<Realtime>>,
  Json(payload): Json<BroadcastMessage>,
) -> Result<ApiResponse<BroadcastMessageResponse>, ApiError> {
  let authorization = headers
    .get(header::AUTHORIZATION)
    .and_then(|value| value.to_str().ok())
    .ok_or_else(|| ApiError::unauthorized("Missing Basic credentials"))?;

  let (_scheme, encoded) = authorization
    .split_once(' ')
    .ok_or_else(|| ApiError::unauthorized("Invalid Basic credentials"))?;

  let decoded = decode_to_string(encoded)
    .map_err(|_| ApiError::unauthorized("Invalid Basic credentials"))?;

  let (key_name, _key_secret) = decoded
    .split_once(':')
    .ok_or_else(|| ApiError::unauthorized("Invalid credentials"))?;

  tracing::debug!("key_name: {:?}", key_name);

  let app_key_name = key_name
    .parse::<ApplicationKeyName>()
    .map_err(|_| ApiError::unauthorized("Invalid credentials"))?;

  let app_id = app_key_name.application_id();

  let application = realtime
    .application(app_id)
    .ok_or_else(|| ApiError::unauthorized("Unknown application"))?;

  let message = Message {
    name: payload.name,
    data: payload.data,
    client_id: None,
  };

  let sent = application.channel_hub
    .broadcast(&channel, ProtocolMessage::message(&channel, vec![message]))
    .await;

  Ok(ApiResponse::new(BroadcastMessageResponse { sent }))
}