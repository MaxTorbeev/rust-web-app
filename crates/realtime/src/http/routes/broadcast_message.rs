use std::sync::Arc;
use axum::extract::{Path, Query, State};
use axum::Json;
use api_response::{ApiError, ApiResponse};
use crate::{ChannelHub, Message, ProtocolMessage, Realtime, RealtimeAccess};
use crate::requests::broadcast_message::BroadcastMessage;
use crate::requests::WebSocketQuery;
use crate::responses::BroadcastMessageResponse;

pub async fn broadcast_message(
  Path(channel): Path<String>,
  Query(query): Query<WebSocketQuery>,
  State(realtime): State<Arc<Realtime>>,
  Json(payload): Json<BroadcastMessage>,
) -> Result<ApiResponse<BroadcastMessageResponse>, ApiError> {

  let RealtimeAccess {
    application,
    token,
  } = realtime
    .verify_access_token(&query.access_token)
    .map_err(|e| {
      ApiError::unauthorized("Invalid access token")
    })?;

  // need to check publish allows

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