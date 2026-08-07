use crate::OutboundSendError;

pub enum SessionEndReason {
  ProtocolLoopFinished(Result<(), OutboundSendError>),
  ShutdownRequested,
  WriterFinished(Result<(), tokio::task::JoinError>),
}