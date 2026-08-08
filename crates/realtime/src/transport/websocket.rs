use std::sync::Arc;
use axum::extract::ws::WebSocket;
use event_bus::EventBus;
use crate::{
  Connection,
  RealtimeApplication,
  WebsocketConnected,
  WebsocketDisconnected,
};
use crate::transport::WebSocketSession;

pub async fn handle_socket(
  socket: WebSocket,
  connection: Connection,
  application: Arc<RealtimeApplication>,
  event_bus: Arc<EventBus>,
) {
  let connection_id = connection.id.as_str().to_owned();

  event_bus
    .emit(WebsocketConnected {
      connection_id: connection_id.clone(),
    })
    .await;

  let result = WebSocketSession::new(socket, connection, application)
    .run()
    .await;

  event_bus
    .emit(WebsocketDisconnected { connection_id })
    .await;

  if let Err(error) = result {
    tracing::error!(?error, "error occurred while handling socket");
  }
}
