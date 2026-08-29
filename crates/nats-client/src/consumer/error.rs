use thiserror::Error;
use crate::AckError;

#[derive(Debug, Error)]
pub enum JetStreamConsumerError {
  #[error("failed to ACK incoming JetStream event")]
  Ack(#[source] AckError),

  #[error("failed to NAK incoming JetStream event")]
  Nak(#[source] AckError),

  #[error("failed to TERM incoming JetStream event")]
  Term(#[source] AckError),
}