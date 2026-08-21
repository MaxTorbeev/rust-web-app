use std::future::Future;
use std::pin::Pin;

use crate::{DeliveryClass, EventBusError, EventMessage};

mod local;

pub use local::LocalEventPublisher;

pub type EventPublishFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(), EventBusError>> + Send + 'a>>;

/// Publishes an already prepared event envelope through one delivery backend.
///
/// Implementations own their retry policy and return only after publication
/// succeeds or reaches a terminal error. Retries must reuse the same message
/// and therefore the same event identifier. Durable backends return success
/// only after their native publish confirmation has been received.
pub trait EventPublisher: Send + Sync {
    fn publish<'a>(
        &'a self,
        message: &'a EventMessage,
        delivery: DeliveryClass,
    ) -> EventPublishFuture<'a>;
}
