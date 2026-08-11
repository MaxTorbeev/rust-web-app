use async_nats::ConnectError as DriverConnectError;
use async_nats::jetstream::context::PublishError as DriverPublishError;
use async_nats::jetstream::context::CreateStreamError as DriverStreamError;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("failed to connect to NATS: {source}")]
pub struct ConnectError {
  #[source]
  source: DriverConnectError,
}

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

impl ConnectError {
  pub(crate) fn from_driver(source: DriverConnectError) -> Self {
    Self { source }
  }
}

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