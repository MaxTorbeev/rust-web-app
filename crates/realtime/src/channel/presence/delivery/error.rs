use crate::BroadcastError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PresenceDeliveryError {
  #[error("invalid committed presence transition: {message}")]
  InvalidTransition { message: String },

  #[error("failed to project committed presence transition: {message}")]
  Projection { message: String },

  #[error("local channel delivery failed: {0}")]
  ChannelDelivery(#[from] BroadcastError),
}
