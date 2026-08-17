use axum::extract::{Path};
use api_response::{ApiError, ApiResponse};
use crate::{Message, ProtocolMessage};
use crate::extractors::{BroadcastApplication, BroadcastMessages};
use crate::responses::BroadcastMessageResponse;

pub async fn broadcast_message(
  Path(channel): Path<String>,
  BroadcastApplication(application): BroadcastApplication,
  payloads: BroadcastMessages,
) -> Result<ApiResponse<BroadcastMessageResponse>, ApiError> {

  let messages:Vec<Message> = payloads
    .into_inner()
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

  let outcome = application.channel_hub
    .broadcast(&channel, protocol_message)
    .await
    .map_err(|error| {
      tracing::error!(%error, %channel, "failed to broadcast HTTP message");

      ApiError::internal("failed to broadcast message")
    })?;

  Ok(ApiResponse::new(BroadcastMessageResponse {
    sent: outcome.enqueued,
  }))
}
