use tokio::task::JoinError;
use thiserror::Error;
use crate::OutboundSendError;
use crate::transport::protocol_reader::ReaderError;

#[derive(Debug, Error)]
pub(crate) enum SessionError {
  #[error("failed to read from websocket: {0}")]
  Read(#[source] axum::Error),

  #[error("failed to enqueue outbound websocket message: {0:?}")]
  Outbound(OutboundSendError),

  #[error("websocket writer stopped unexpectedly")]
  WriterStopped,

  #[error("failed to write to websocket: {0}")]
  Write(#[source] axum::Error),

  #[error("websocket writer task failed: {0}")]
  WriterTaskFailed(#[source] JoinError),

  #[error("websocket writer drain timed out")]
  WriterDrainTimedOut
}

impl From<ReaderError> for SessionError {
  fn from(error: ReaderError) -> Self {
    match error {
      ReaderError::Read(error) => Self::Read(error),
      ReaderError::Outbound(error) => Self::Outbound(error),
    }
  }
}

impl From<OutboundSendError> for SessionError {
  fn from(error: OutboundSendError) -> Self {
    Self::Outbound(error)
  }
}
