use std::sync::Arc;
use axum::extract::{Path, State};
use api_response::{ApiError, ApiResponse};
use event_bus::EventBus;
use crate::{ChannelMessageSubmitted, Message};
use crate::extractors::{BroadcastApplication, BroadcastMessages};
use crate::responses::BroadcastMessageResponse;

pub async fn broadcast_message(
  Path(channel): Path<String>,
  BroadcastApplication(application): BroadcastApplication,
  State(event_bus): State<Arc<EventBus>>,
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

  let receipt = event_bus
    .publish(ChannelMessageSubmitted {
      application_id: application.id.clone(),
      channel: channel.clone(),
      messages,
    })
    .await
    .map_err(|e| {
      tracing::error!(%e, %channel, "failed to publish HTTP broadcast event");

      ApiError::internal("failed to broadcast message")
    })?;

  Ok(ApiResponse::new(BroadcastMessageResponse {
    accepted: true,
    event_id: receipt.event_id,
  }))
}
