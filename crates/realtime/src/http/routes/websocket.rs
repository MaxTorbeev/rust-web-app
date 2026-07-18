use axum::extract::ws::WebSocketUpgrade;
use axum::response::{IntoResponse, Response};
use api_response::{ApiError};
use crate::websocket::handle_socket;

pub async fn websocket(
  ws: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    Ok(ws.on_upgrade(|_socket| async move {
      handle_socket(_socket).await
    }).into_response())
}
