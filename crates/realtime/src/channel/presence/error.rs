use crate::{ChannelCommitDeliveryError, PresenceStoreError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PresenceError {
  #[error("presence store failed: {0}")]
  Store(#[from] PresenceStoreError),

  #[error("presence delivery failed: {0}")]
  Delivery(#[from] ChannelCommitDeliveryError),
}
