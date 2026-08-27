use thiserror::Error;

/// Error returned when a JetStream delivery cannot be acknowledged or settled.
///
/// Ошибка, возникающая, когда доставку JetStream не удалось подтвердить или
/// завершить с нужным статусом.
#[derive(Debug, Error)]
#[error("failed to settle JetStream message: {source}")]
pub struct AckError {
  #[source]
  source: async_nats::Error,
}

impl AckError {
  pub(crate) fn from_driver(source: async_nats::Error) -> Self {
    Self { source }
  }
}
