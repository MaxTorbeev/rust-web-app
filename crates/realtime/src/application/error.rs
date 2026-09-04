use crate::PresenceError;

#[derive(Debug, thiserror::Error)]
pub enum ConnectionCleanupError {
  #[error("failed to clean up channel state: {0}")]
  ChannelState(#[from] PresenceError),
}