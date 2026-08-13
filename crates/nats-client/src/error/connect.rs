use thiserror::Error;
use async_nats::ConnectError as DriverConnectError;

#[derive(Debug, Error)]
#[error("failed to connect to NATS: {source}")]
pub struct ConnectError {
  #[source]
  source: DriverConnectError,
}

impl ConnectError {
  pub(crate) fn from_driver(source: DriverConnectError) -> Self {
    Self { source }
  }
}