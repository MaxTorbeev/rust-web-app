use thiserror::Error;

#[derive(Debug, Error)]
#[error("Failed to acknowledge JetStream message: {source}")]
pub struct AckError {
  #[source]
  source: async_nats::Error,
}

impl AckError {
  pub(crate) fn from_driver(source: async_nats::Error) -> Self {
    Self { source }
  }
}