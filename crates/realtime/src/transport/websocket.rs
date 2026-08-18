use std::sync::Arc;
use axum::extract::ws::WebSocket;
use event_bus::{EventBus, EventBusError};
use thiserror::Error;
use crate::{
  Connection,
  RealtimeApplication,
  WebsocketConnected,
  WebsocketDisconnected,
};
use crate::transport::{SessionError, WebSocketSession};

#[derive(Debug, Error)]
enum HandleSocketError {
  #[error("websocket lifecycle event failed: {0}")]
  EventBus(#[from] EventBusError),

  #[error("websocket session failed: {0}")]
  Session(#[from] SessionError),
}

pub async fn handle_socket(
  socket: WebSocket,
  connection: Connection,
  application: Arc<RealtimeApplication>,
  event_bus: Arc<EventBus>,
) {
  if let Err(error) = run_socket(
    socket,
    connection,
    application,
    event_bus,
  ).await {
    tracing::error!(%error, "websocket handling failed");
  }
}

async fn run_socket(
  socket: WebSocket,
  connection: Connection,
  application: Arc<RealtimeApplication>,
  event_bus: Arc<EventBus>,
) -> Result<(), HandleSocketError> {
  let connection_id = connection.id.as_str().to_owned();

  event_bus
    .publish(WebsocketConnected {
      connection_id: connection_id.clone(),
    })
    .await?;

  let session_result = WebSocketSession::new(
    socket,
    connection,
    application,
    event_bus.clone(),
  )
    .run()
    .await;

  let disconnected_result = event_bus
    .publish(WebsocketDisconnected { connection_id })
    .await;

  // Disconnected уже был опубликован,
  // даже если сессия завершилась ошибкой.
  session_result?;
  disconnected_result?;

  Ok(())
}
