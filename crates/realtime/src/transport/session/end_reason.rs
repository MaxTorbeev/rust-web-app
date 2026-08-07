use tokio::task::JoinError;
use crate::OutboundSendError;

pub enum EndReason {
  Graceful,
  /// Был принять запрос на остановку Вебсокет сессии
  ShutdownRequested,
  ProtocolFailed(OutboundSendError),
  /// Writer был остановлен
  WriterStopped(Result<(), JoinError>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriterPolicy {
  DrainUntilShutdown,
  Abort,
  AlreadyStopped
}

impl EndReason {
  pub(crate) fn writer_policy(&self) -> WriterPolicy {
    match self {
      Self::Graceful => WriterPolicy::DrainUntilShutdown,
      Self::ShutdownRequested => WriterPolicy::Abort,
      Self::ProtocolFailed(_) => WriterPolicy::Abort,
      Self::WriterStopped(_) => WriterPolicy::AlreadyStopped
    }
  }


  /// Преобразовать причину остановки в Result
  pub(crate) fn into_result(self) -> Result<(), OutboundSendError> {
    match self {
      Self::Graceful => Ok(()),
      // Управляемое завершение slow connection.
      Self::ShutdownRequested => Ok(()),
      // Возвращаем исходную ошибку protocol loop.
      Self::ProtocolFailed(error) => {
        Err(error)
      }
      // Writer завершился раньше reader-а.
      // Отправлять данные в WebSocket больше нельзя.
      Self::WriterStopped(writer_result) => {
        if let Err(error) = writer_result {
          tracing::error!(%error, "websocket writer task failed");
        }

        Err(OutboundSendError::QueueClosed)
      }
    }
  }
}