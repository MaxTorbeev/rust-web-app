use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use crate::{Event, EventBusError, EventMessage};

type EventHandlerFuture = Pin<Box<dyn Future<Output = Result<(), EventBusError>> + Send + 'static>>;

type EventHandler =
    Box<dyn Fn(&EventMessage) -> Result<EventHandlerFuture, EventBusError> + Send + Sync + 'static>;

#[derive(Default)]
pub struct EventDispatcher {
    handlers: HashMap<&'static str, EventHandler>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the single required handler responsible for an event type.
    ///
    /// All handlers must be registered before the dispatcher is shared.
    pub fn register<E, F, Fut>(&mut self, handler: F) -> Result<(), EventBusError>
    where
        E: Event,
        F: Fn(E) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), EventBusError>> + Send + 'static,
    {
        if self.handlers.contains_key(E::NAME) {
            return Err(EventBusError::HandlerAlreadyRegistered {
                event_name: E::NAME.to_owned(),
            });
        }

        self.handlers.insert(
            E::NAME,
            Box::new(move |message| {
                let event = message.decode_event::<E>()?;
                let future: EventHandlerFuture = Box::pin(handler(event));

                Ok(future)
            }),
        );

        Ok(())
    }

    /// Applies an already received event message to the local handler.
    pub async fn dispatch(&self, message: &EventMessage) -> Result<(), EventBusError> {
        let handler = self.handlers.get(message.event_name()).ok_or_else(|| {
            EventBusError::HandlerNotRegistered {
                event_name: message.event_name().to_owned(),
            }
        })?;

        handler(message)?.await
    }
}
