use async_nats::jetstream::context::PublishError as DriverPublishError;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("failed to publish JetStream message: {source}")]
pub struct PublishError {
  #[source]
  source: DriverPublishError,
}

impl PublishError {
  pub(crate) fn from_driver(source: DriverPublishError) -> Self {
    Self { source }
  }
}