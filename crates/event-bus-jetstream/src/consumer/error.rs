use nats_client::{AckError, ReceiveError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum JetStreamConsumerError {
  #[error("failed to receive incoming JetStream event")]
  Receive(#[source] ReceiveError),

  #[error("JetStream subscription closed unexpectedly")]
  SubscriptionClosed,

  #[error("failed to ACK incoming JetStream event")]
  Ack(#[source] AckError),

  #[error("failed to NAK incoming JetStream event")]
  Nak(#[source] AckError),

  #[error("failed to TERM incoming JetStream event")]
  Term(#[source] AckError),
}
