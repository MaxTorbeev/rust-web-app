use crate::transport::{ProtocolOutcome, SocketContext, handle_protocol_message};
use crate::{Connection, OutboundSendError, OutboundSender, ProtocolMessage, RealtimeApplication};
use axum::Error;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use event_bus::EventBus;
use futures_util::StreamExt;
use futures_util::stream::SplitStream;

pub(crate) type ReaderResult = Result<ReaderEndReason, ReaderError>;

pub struct ProtocolReader {
  stream: SplitStream<WebSocket>,
}

pub(crate) enum ReaderEndReason {
  /// Был запрос на завершение стрима
  DisconnectRequested,
  /// Был сигнал на закрытие сокетов
  SocketClosed,
  /// Стрим завешен штатно
  StreamEnded,
}

#[derive(Debug)]
pub(crate) enum ReaderError {
  Read(Error),
  Outbound(OutboundSendError),
}

impl ProtocolReader {
  pub fn new(stream: SplitStream<WebSocket>) -> Self {
    Self { stream }
  }
  pub(in crate::transport::session) async fn run(
    &mut self,
    sender: &OutboundSender,
    connection: &Connection,
    application: &RealtimeApplication,
    event_bus: &EventBus,
  ) -> ReaderResult {
    while let Some(result) = self.stream.next().await {
      let frame = match result {
        Ok(message) => message,
        Err(error) => {
          tracing::error!(%error, "websocket read failed");

          return Err(ReaderError::Read(error));
        }
      };

      match frame {
        Message::Text(text) => {
          let message = match serde_json::from_str::<ProtocolMessage>(&text) {
            Ok(m) => m,
            Err(e) => {
              tracing::error!(%e, "invalid websocket message");

              let response = ProtocolMessage::nack(None);

              if let Err(e) = sender.try_enqueue_protocol_message(&response) {
                tracing::error!(?e, "failed to enqueue protocol message");

                return Err(ReaderError::Outbound(e));
              }

              continue;
            }
          };

          let context = SocketContext {
            connection: &connection,
            sender: &sender,
            router: application.router(),
            attachments: application.attachments(),
            presence: application.presence(),
            event_bus,
          };

          let ProtocolOutcome {
            replies,
            disconnect,
          } = handle_protocol_message(message, &context).await;

          for reply in replies {
            if let Err(e) = sender.try_enqueue_protocol_message(&reply) {
              tracing::error!(?e, "failed to enqueue protocol message");

              return Err(ReaderError::Outbound(e));
            }
          }

          if disconnect {
            return Ok(ReaderEndReason::DisconnectRequested);
          }
        }
        Message::Close(_) => {
          return Ok(ReaderEndReason::SocketClosed);
        }
        _ => {}
      }
    }

    Ok(ReaderEndReason::StreamEnded)
  }
}
