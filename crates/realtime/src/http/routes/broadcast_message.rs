use std::sync::Arc;
use axum::extract::{Path, State};
use axum::Json;
use api_response::{ApiResponse};
use crate::{ChannelHub, Message, ProtocolMessage};
use crate::requests::broadcast_message::BroadcastMessage;
use crate::responses::BroadcastMessageResponse;

pub async fn broadcast_message(
  Path(channel): Path<String>,
  State(channel_hub): State<Arc<ChannelHub>>,
  Json(payload): Json<BroadcastMessage>
) -> ApiResponse<BroadcastMessageResponse> {
  let message = Message {
    name: payload.name,
    data: payload.data,
    client_id: None,
  };

  let sent = channel_hub
    .broadcast(&channel, ProtocolMessage::message(&channel, vec![message]))
    .await;

  ApiResponse::new(BroadcastMessageResponse {sent})
}