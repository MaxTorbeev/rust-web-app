use axum::extract::ws::WebSocket;
use futures_util::stream::SplitStream;
use crate::{Connection, OutboundSendError, OutboundSender, ProtocolMessage, RealtimeApplication};
use crate::transport::{handle_protocol_message, ProtocolOutcome, SocketContext};
use axum::extract::ws::{Message};
use futures_util::StreamExt;

pub struct ProtocolReader {
  stream: SplitStream<WebSocket>,
}

impl ProtocolReader {
  pub fn new(stream: SplitStream<WebSocket>) -> Self {
    Self {
      stream
    }
  }
  pub async fn run(
    &mut self,
    sender: &OutboundSender,
    connection: &Connection,
    application: &RealtimeApplication,
  ) -> Result<(), OutboundSendError> {
    while let Some(result) = self.stream.next().await {
      let frame = match result {
        Ok(message) => message,
        Err(error) => {
          tracing::error!(%error, "websocket read failed");

          return Ok(());
        }
      };

      match frame {
        Message::Text(text) => {
          let message = match serde_json::from_str::<ProtocolMessage>(&text) {
            Ok(m) => m,
            Err(e) => {
              tracing::error!(%e, "invalid websocket message");

              let response = ProtocolMessage::nack(None);

              sender.try_enqueue_protocol_message(&response)?;

              continue
            }
          };

          let context = SocketContext {
            connection: &connection,
            sender: &sender,
            presence_hub: &application.presence_hub,
            channel_hub: &application.channel_hub,
          };

          let ProtocolOutcome {
            replies,
            disconnect
          } = handle_protocol_message(message, &context).await;

          for reply in replies {
            sender.try_enqueue_protocol_message(&reply)?;
          }

          if disconnect {
            return Ok(());
          }
        }
        Message::Close(_) => {
          return Ok(());
        }
        _ => {}
      }
    }

    Ok(())
  }
}