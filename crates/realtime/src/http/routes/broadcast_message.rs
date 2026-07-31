use axum::extract::{Path};
use axum::Json;
use api_response::{ApiError, ApiResponse};
use crate::{Message, ProtocolMessage};
use crate::extractors::BroadcastApplication;
use crate::requests::broadcast_message::BroadcastMessage;
use crate::responses::BroadcastMessageResponse;

pub async fn broadcast_message(
  Path(channel): Path<String>,
  BroadcastApplication(application): BroadcastApplication,
  Json(payload): Json<BroadcastMessage>,
) -> Result<ApiResponse<BroadcastMessageResponse>, ApiError> {
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