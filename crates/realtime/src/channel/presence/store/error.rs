use thiserror::Error;

#[derive(Debug, Error)]
pub enum PresenceStoreError {
  #[error("invalid request: {message}")]
  InvalidRequest { message: String },
  #[error("operation conflict: {message}")]
  Conflict { message: String },
  #[error("internal store error: {message}")]
  Internal { message: String },
}
