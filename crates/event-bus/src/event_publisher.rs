use std::future::Future;
use std::pin::Pin;

use crate::{EventBusError, EventMessage};

pub type EventPublishFuture<'a> =
  Pin<Box<dyn Future<Output = Result<(), EventBusError>> + Send + 'a>>;

/// Publishes an already prepared event envelope through one delivery backend.
///
/// Implementations own their retry policy and return only after the publication
/// either succeeds or reaches a terminal error. A retry must reuse the same
/// `EventMessage` and therefore the same event identifier. A JetStream-backed
/// implementation must return success only after receiving the publish ACK.
pub trait EventPublisher: Send + Sync {
  fn publish<'a>(&'a self, message: &'a EventMessage) -> EventPublishFuture<'a>;
}
