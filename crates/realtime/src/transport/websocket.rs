use axum::extract::ws::{Message as WsMessage, Message, WebSocket};
use tracing::error;
use crate::{ProtocolAction, ProtocolMessage};

pub async fn handle_socket(mut socket: WebSocket) {
  tracing::info!("websocket connected");

  while let Some(result) = socket.recv().await {
    tracing::debug!(?result, "websocket message");

    let ws_message = match result {
      Ok(msg) => msg,
      Err(e) => {
        tracing::error!(%e, "websocket read failed");
        break;
      }
    };

    match ws_message {
      WsMessage::Text(text) => {
        let message = match serde_json::from_str::<ProtocolMessage>(&text) {
          Ok(m) => m,
          Err(e) => {
            tracing::error!(%e, "invalid websocket message");
            break;
          }
        };

        let response = match message.action {
          ProtocolAction::Connect => ProtocolMessage::connected(),
          _ => continue,
        };

        send_protocol_message(&mut socket, response).await;
      }
      _ => {}
    }
  }

  tracing::info!("websocket disconnected");
}

async fn send_protocol_message(
  socket: &mut WebSocket,
  message: ProtocolMessage
) -> Result<(), axum::Error> {
  let text = serde_json::to_string(&message).unwrap();

  socket.send(WsMessage::Text(text.into())).await
}