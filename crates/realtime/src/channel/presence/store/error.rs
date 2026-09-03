use thiserror::Error;

#[derive(Debug, Error)]
pub enum PresenceStoreError {
  #[error("invalid request: {message}")]
  InvalidRequest { message: String },
  #[error("entry not found: {message}")]
  NotFound { message: String },
  #[error("duplicate operation: {message}")]
  Duplicate { message: String },
  #[error("operation conflict: {message}")]
  Conflict { message: String },
  #[error("protocol conflict")]
  ProtocolConflict,
  #[error("internal store error: {message}")]
  Internal { message: String },
}
