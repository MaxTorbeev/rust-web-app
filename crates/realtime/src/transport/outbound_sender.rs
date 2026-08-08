use tokio::sync::{mpsc, watch};

use crate::{PreparedFrame, ProtocolMessage};
use crate::transport::ShutdownTrigger;

/// Дескриптор исходящей очереди одного WebSocket-соединения.
///
/// Преобразует одиночные `ProtocolMessage` в готовые `PreparedFrame` и
/// добавляет уже подготовленные broadcast-frame без повторной сериализации.
#[derive(Clone)]
pub struct OutboundSender {
  inner: mpsc::Sender<PreparedFrame>,

  /// Передаёт WebSocket-сессии сигнал о необходимости
  /// принудительно завершить соединение.
  shutdown: ShutdownTrigger
}

#[derive(Debug)]
pub enum OutboundSendError {
  Serialization(serde_json::Error),
  QueueClosed,
  QueueFull
}


impl OutboundSender {
  pub fn new(inner: mpsc::Sender<PreparedFrame>, shutdown: ShutdownTrigger) -> Self {
    Self {
      inner,
      shutdown
    }
  }

  /// Сериализовать обычный единичный ответ и поставить в очередь;
  pub fn try_enqueue_protocol_message(&self, message: &ProtocolMessage) -> Result<(), OutboundSendError> {
    let frame = PreparedFrame::try_from(message)
      .map_err(OutboundSendError::Serialization)?;

    self.try_enqueue_prepared_frame(frame)
  }

  pub fn request_shutdown(&self) {
    self.shutdown.request();
  }

  pub fn try_enqueue_prepared_frame(&self, frame: PreparedFrame) -> Result<(), OutboundSendError> {
    match self.inner.try_send(frame) {
      Ok(()) => Ok(()),

      Err(mpsc::error::TrySendError::Full(_)) => {
        Err(OutboundSendError::QueueFull)
      }

      Err(mpsc::error::TrySendError::Closed(_)) => {
        Err(OutboundSendError::QueueClosed)
      }
    }
  }
}
