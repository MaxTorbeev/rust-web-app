use axum::extract::ws::{Message as WsMessage, WebSocket};
use crate::{Connection, ProtocolAction, ProtocolMessage};

pub async fn handle_socket(mut socket: WebSocket, connection: Connection) {
  tracing::info!("websocket connected");

  while let Some(result) = socket.recv().await {
    tracing::debug!(?result, "websocket message");

    let ws_message = match result {
      Ok(msg) => msg,
      Err(_e) => {
        tracing::error!(%_e, "websocket read failed");
        break;
      }
    };

    match ws_message {
      WsMessage::Text(text) => {
        let message = match serde_json::from_str::<ProtocolMessage>(&text) {
          Ok(m) => m,
          Err(e) => {
            tracing::error!(%e, "invalid websocket message");

            let response = ProtocolMessage::nack(None);

            if let Err(_e) = send_protocol_message(&mut socket, response).await {
              tracing::error!(?e, "websocket send failed");
              break;
            }

            continue
          }
        };

        let response = match message.action {
          ProtocolAction::Connect => ProtocolMessage::connected(&connection),
          ProtocolAction::Attach => ProtocolMessage::attached(&message),
          ProtocolAction::Presence => ProtocolMessage::ack(&message),
          ProtocolAction::Message => ProtocolMessage::ack(&message),
          ProtocolAction::Heartbeat => ProtocolMessage::heartbeat(),
          _ => continue,
        };

        if let Err(err) = send_protocol_message(&mut socket, response).await {
          tracing::error!(?err, "websocket send error");

          break;
        }
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