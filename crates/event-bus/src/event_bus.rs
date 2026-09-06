use std::sync::Arc;

use crate::{
  DeliveryClass, Event, EventBusError, EventDispatcher, EventMessage, EventPublisher,
  LocalEventPublisher, PublishReceipt,
};

pub struct EventBus {
  local_publisher: Arc<dyn EventPublisher>,
  distributed_publisher: Arc<dyn EventPublisher>,
}

impl EventBus {
  /// Creates a bus that applies every delivery class in the current process.
  pub fn local(dispatcher: Arc<EventDispatcher>) -> Self {
    let publisher: Arc<dyn EventPublisher> = Arc::new(LocalEventPublisher::new(dispatcher));

    Self {
      local_publisher: Arc::clone(&publisher),
      distributed_publisher: publisher,
    }
  }

  /// Creates a bus that applies local events inline and sends distributed
  /// events through the configured transport publisher.
  pub fn with_distributed_publisher(
    dispatcher: Arc<EventDispatcher>,
    distributed_publisher: Arc<dyn EventPublisher>,
  ) -> Self {
    let local_publisher: Arc<dyn EventPublisher> = Arc::new(LocalEventPublisher::new(dispatcher));

    Self {
      local_publisher,
      distributed_publisher,
    }
  }

  /// Creates one transport-independent message and publishes it according to
  /// the event's delivery class.
  ///
  /// Every call mints a fresh `event_id`, so a retry of `publish` is a new
  /// event for every consumer. Callers that must republish an already
  /// committed event use [`EventBus::publish_message`] instead.
  pub async fn publish<E>(&self, event: E) -> Result<PublishReceipt, EventBusError>
  where
    E: Event,
  {
    let message = EventMessage::try_from_event(&event)?;

    self.publish_message(&message, E::DELIVERY).await
  }

  /// Publishes an already prepared envelope, keeping its `event_id`.
  ///
  /// This is the publication path for events whose identity was fixed before
  /// they reached the bus: an outbox record written atomically together with a
  /// state transition, or a committed event that is being re-delivered after a
  /// failed attempt. Consumers deduplicate by `event_id` (JetStream additionally
  /// uses it as `Nats-Msg-Id`), so a retry through this method is idempotent
  /// for them, while the same retry through [`EventBus::publish`] would produce
  /// a duplicate under a new identifier.
  ///
  /// The caller is responsible for choosing the delivery class the event was
  /// declared with; the bus cannot recover it from the envelope alone. The
  /// envelope is passed by reference on purpose: the caller keeps the exact
  /// bytes it stored and can retry with them unchanged.
  pub async fn publish_message(
    &self,
    message: &EventMessage,
    delivery: DeliveryClass,
  ) -> Result<PublishReceipt, EventBusError> {
    let publisher = match delivery {
      DeliveryClass::LocalOnly => &self.local_publisher,
      DeliveryClass::AllNodes | DeliveryClass::WorkQueue => &self.distributed_publisher,
    };

    publisher.publish(message, delivery).await?;

    Ok(PublishReceipt::new(message.event_id()))
  }
}
