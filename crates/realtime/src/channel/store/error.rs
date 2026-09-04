use thiserror::Error;

/// Ошибка хранилища состояния каналов.
#[derive(Debug, Error)]
pub enum ChannelStateStoreError {
  #[error("invalid request: {message}")]
  InvalidRequest { message: String },

  #[error("operation conflict: {message}")]
  Conflict { message: String },

  #[error("internal channel state store error: {message}")]
  Internal { message: String },
}
