use tokio::sync::{mpsc, watch};

use crate::transport::ShutdownTrigger;
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
  shutdown: ShutdownTrigger,
}

#[derive(Debug)]
pub enum OutboundSendError {
  Serialization(serde_json::Error),
  QueueClosed,
  QueueFull,
}

impl OutboundSender {
  pub fn new(inner: mpsc::Sender<PreparedFrame>, shutdown: ShutdownTrigger) -> Self {
    Self { inner, shutdown }
  }

  /// Сериализовать обычный единичный ответ и поставить в очередь;
  pub fn try_enqueue_protocol_message(
    &self,
    message: &ProtocolMessage,
  ) -> Result<(), OutboundSendError> {
    let frame = PreparedFrame::try_from(message).map_err(OutboundSendError::Serialization)?;

    self.try_enqueue_prepared_frame(frame)
  }

  pub fn request_shutdown(&self) {
    self.shutdown.request();
  }

  pub fn try_enqueue_prepared_frame(&self, frame: PreparedFrame) -> Result<(), OutboundSendError> {
    match self.inner.try_send(frame) {
      Ok(()) => Ok(()),

      Err(mpsc::error::TrySendError::Full(_)) => Err(OutboundSendError::QueueFull),

      Err(mpsc::error::TrySendError::Closed(_)) => Err(OutboundSendError::QueueClosed),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::transport::shutdown_channel;

  fn test_sender(capacity: usize) -> (OutboundSender, mpsc::Receiver<PreparedFrame>) {
    let (shutdown_trigger, _shutdown_listener) = shutdown_channel();
    let (queue_sender, queue_receiver) = mpsc::channel(capacity);

    (
      OutboundSender::new(queue_sender, shutdown_trigger),
      queue_receiver,
    )
  }

  fn heartbeat_frame() -> PreparedFrame {
    PreparedFrame::try_from(&ProtocolMessage::heartbeat()).expect("heartbeat must be serializable")
  }

  #[test]
  fn returns_queue_full_when_capacity_is_exhausted() {
    let (sender, _receiver) = test_sender(1);

    sender
      .try_enqueue_prepared_frame(heartbeat_frame())
      .expect("the first frame must fit into the queue");

    let result = sender.try_enqueue_prepared_frame(heartbeat_frame());

    assert!(matches!(result, Err(OutboundSendError::QueueFull)));
  }

  #[test]
  fn returns_queue_closed_when_writer_is_gone() {
    let (sender, receiver) = test_sender(1);

    // Dropping the receiver models a writer task that has already stopped.
    drop(receiver);

    let result = sender.try_enqueue_prepared_frame(heartbeat_frame());

    assert!(matches!(result, Err(OutboundSendError::QueueClosed)));
  }
}
