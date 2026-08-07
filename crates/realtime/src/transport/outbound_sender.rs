use tokio::sync::{mpsc, watch};

use crate::{PreparedFrame, ProtocolMessage};

/// Дескриптор исходящей очереди одного WebSocket-соединения.
///
/// Преобразует одиночные `ProtocolMessage` в готовые `PreparedFrame` и
/// добавляет уже подготовленные broadcast-frame без повторной сериализации.
#[derive(Clone)]
pub struct OutboundSender {
  inner: mpsc::Sender<PreparedFrame>,

  /// Передаёт WebSocket-сессии сигнал о необходимости
  /// принудительно завершить соединение.
  shutdown_signal: watch::Sender<bool>
}

#[derive(Debug)]
pub enum OutboundSendError {
  Serialization(serde_json::Error),
  QueueClosed,
  QueueFull
}


impl OutboundSender {
  pub fn new(
    inner: mpsc::Sender<PreparedFrame>,
    shutdown_signal: watch::Sender<bool>
  ) -> Self {
    Self {
      inner,
      shutdown_signal
    }
  }

  /// Сериализовать обычный единичный ответ и поставить в очередь;
  pub fn try_enqueue_protocol_message(&self, message: &ProtocolMessage) -> Result<(), OutboundSendError> {
    let frame = PreparedFrame::try_from(message)
      .map_err(OutboundSendError::Serialization)?;

    self.try_enqueue_prepared_frame(frame)
  }

  pub fn request_shutdown(&self) {
    self.shutdown_signal.send_replace(true);
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
