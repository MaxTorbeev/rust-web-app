use crate::PresenceAction;
use thiserror::Error;

#[derive(Debug, Error)]
pub(super) enum PresenceRequestError {
  #[error("presence message does not contain a channel")]
  MissingChannel,

  #[error("presence message does not contain msgSerial")]
  MissingMessageSerial,

  #[error("presence batch is empty")]
  EmptyBatch,

  #[error("presence action {0:?} cannot change presence state")]
  UnsupportedAction(PresenceAction),

  #[error("failed to normalize presence request: {0}")]
  Normalization(#[from] serde_json::Error),
}
