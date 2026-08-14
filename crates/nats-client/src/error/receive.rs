use thiserror::Error;

use async_nats::jetstream::consumer::pull::{
  MessagesError as DriverReceiveError,
};

#[derive(Debug, Error)]
#[error("Failed to receive JetStream message: {source}")]
pub struct ReceiveError {
  #[source]
  source: DriverReceiveError,
}

impl ReceiveError {
  pub(crate) fn from_driver(source: DriverReceiveError) -> Self {
    Self { source }
  }
}