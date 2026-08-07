use std::sync::Arc;
use axum::extract::ws::{WebSocket};
use crate::{Connection, RealtimeApplication};
use crate::transport::WebSocketSession;

pub async fn handle_socket(
  socket: WebSocket,
  connection: Connection,
  application: Arc<RealtimeApplication>,
) {
  let result =  WebSocketSession::new(socket, connection, application)
    .run()
    .await;

  if let Err(error) = result {
    tracing::error!(?error, "error occurred while handling socket");
  }
}