use crate::BroadcastError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChannelCommitDeliveryError {
  #[error("invalid committed channel transition: {message}")]
  InvalidTransition { message: String },

  #[error("failed to project committed channel transition: {message}")]
  Projection { message: String },

  #[error("local channel delivery failed: {0}")]
  LocalDelivery(#[from] BroadcastError),
}
