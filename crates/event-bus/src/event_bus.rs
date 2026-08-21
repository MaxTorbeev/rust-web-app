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
    pub async fn publish<E>(&self, event: E) -> Result<PublishReceipt, EventBusError>
    where
        E: Event,
    {
        let message = EventMessage::try_from_event(&event)?;
        let publisher = match E::DELIVERY {
            DeliveryClass::LocalOnly => &self.local_publisher,
            DeliveryClass::AllNodes | DeliveryClass::WorkQueue => &self.distributed_publisher,
        };

        publisher.publish(&message, E::DELIVERY).await?;

        Ok(PublishReceipt::new(message.event_id()))
    }
}
