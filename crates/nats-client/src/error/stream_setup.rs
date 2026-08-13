use thiserror::Error;
use async_nats::jetstream::context::CreateStreamError as DriverStreamError;

#[derive(Debug, Error)]
#[error("failed to prepare JetStream stream: {source}")]
pub struct StreamSetupError {
  #[source]
  source: DriverStreamError,
}

impl StreamSetupError {
  pub(crate) fn from_driver(source: DriverStreamError) -> Self {
    Self { source }
  }
}
