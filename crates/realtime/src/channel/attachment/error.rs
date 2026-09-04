use crate::{ChannelCommitDeliveryError, PresenceStoreError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AttachmentError {
  #[error("attachment store operation failed: {0}")]
  Store(#[from] PresenceStoreError),

  #[error("attachment transition delivery failed: {0}")]
  Delivery(#[from] ChannelCommitDeliveryError),
}
