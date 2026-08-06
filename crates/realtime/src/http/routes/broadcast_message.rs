use axum::extract::{Path};
use api_response::{ApiError, ApiResponse};
use crate::{Message, ProtocolMessage};
use crate::extractors::{BroadcastApplication, BroadcastMessages};
use crate::responses::BroadcastMessageResponse;

pub async fn broadcast_message(
  Path(channel): Path<String>,
  BroadcastApplication(application): BroadcastApplication,
  BroadcastMessages(payloads): BroadcastMessages,
) -> Result<ApiResponse<BroadcastMessageResponse>, ApiError> {
  let messages:Vec<Message> = payloads
    .into_iter()
    .map(|payload| Message {
      name: payload.name,
      data: payload.data,
      client_id: None,
    })
    .collect();

  let protocol_message = ProtocolMessage::message(
    &channel,
    messages,
  );

  let sent = application.channel_hub
    .broadcast(&channel, protocol_message)
    .await;

  Ok(ApiResponse::new(BroadcastMessageResponse { sent }))
}