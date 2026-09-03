use thiserror::Error;

#[derive(Debug, Error)]
pub enum BroadcastError {
  #[error("failed to serialize broadcast frame: {0}")]
  SerializeFrame(#[from] serde_json::Error),
}
