use std::sync::Arc;

use crate::{DeliveryClass, EventDispatcher, EventMessage, EventPublishFuture, EventPublisher};

pub struct LocalEventPublisher {
    dispatcher: Arc<EventDispatcher>,
}

impl LocalEventPublisher {
    pub fn new(dispatcher: Arc<EventDispatcher>) -> Self {
        Self { dispatcher }
    }
}

impl EventPublisher for LocalEventPublisher {
    fn publish<'a>(
        &'a self,
        message: &'a EventMessage,
        _delivery: DeliveryClass,
    ) -> EventPublishFuture<'a> {
        Box::pin(async move {
            self.dispatcher.dispatch(message).await
        })
    }
}
