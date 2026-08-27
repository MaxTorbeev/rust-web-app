use async_nats::ConnectError as DriverConnectError;
use thiserror::Error;

/// Error returned when a connection to a NATS server cannot be established.
///
/// Ошибка, возникающая, когда не удалось установить соединение с сервером
/// NATS.
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
