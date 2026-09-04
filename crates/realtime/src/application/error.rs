use crate::AttachmentError;

#[derive(Debug, thiserror::Error)]
pub enum ConnectionCleanupError {
  #[error("failed to clean up channel attachments: {0}")]
  Attachment(#[from] AttachmentError),
}