use axum::extract::ws::WebSocket;
pub async fn handle_socket(mut socket: WebSocket) {
  tracing::info!("websocket connected");

  while let Some(result) = socket.recv().await {
    tracing::debug!(?result, "websocket message");
  }

  tracing::info!("websocket disconnected");
}