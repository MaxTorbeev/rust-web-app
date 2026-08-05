use tokio::sync::mpsc;

use crate::{PreparedFrame, ProtocolMessage};

/// Дескриптор исходящей очереди одного WebSocket-соединения.
///
/// Преобразует одиночные `ProtocolMessage` в готовые `PreparedFrame` и
/// добавляет уже подготовленные broadcast-frame без повторной сериализации.
#[derive(Clone)]
pub struct OutboundSender {
  inner: mpsc::Sender<PreparedFrame>,
}

#[derive(Debug)]
pub enum OutboundSendError {
  Serialization(serde_json::Error),
  QueueClosed
}


impl OutboundSender {
  pub fn new(inner: mpsc::Sender<PreparedFrame>) -> Self {
    Self { inner }
  }

  /// Сериализовать обычный единичный ответ и поставить в очередь;
  pub async fn send_protocol(&self, message: &ProtocolMessage) -> Result<(), OutboundSendError> {
    let frame = PreparedFrame::try_from(message)
      .map_err(OutboundSendError::Serialization)?;

    self.send_prepared(frame).await
  }

  pub async fn send_prepared(&self, frame: PreparedFrame) -> Result<(), OutboundSendError> {
    self.inner.send(frame).await.map_err(|_| OutboundSendError::QueueClosed)
  }
}
