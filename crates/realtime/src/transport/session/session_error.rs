use tokio::task::JoinError;
use crate::OutboundSendError;
use crate::transport::protocol_reader::ReaderError;

#[derive(Debug)]
pub(crate) enum SessionError {
  Read(axum::Error),
  Outbound(OutboundSendError),
  WriterStopped,
  Write(axum::Error),
  WriterTaskFailed(JoinError),
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