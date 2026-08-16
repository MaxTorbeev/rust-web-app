use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::EventBusError;

pub trait Event: Send
+ Sync
+ Serialize
+ DeserializeOwned
+ 'static {
  const NAME: &'static str;
  const VERSION: u16 = 1;
}

type EventHandlerFuture = Pin<
  Box<dyn Future<Output = Result<(), EventBusError>> + Send>
>;
type EventHandler = Box<
  dyn Fn(Box<dyn Any + Send>) -> EventHandlerFuture + Send + Sync
>;

#[derive(Default)]
pub struct EventBus {
  handlers: HashMap<TypeId, EventHandler>,
}

impl EventBus {
  pub fn new() -> Self {
    Self::default()
  }

  /// Registers the single handler responsible for an event type.
  ///
  /// All handlers are registered before the bus is shared with the application.
  pub fn register<E, F, Fut>(&mut self, handler: F) -> Result<(), EventBusError>
  where
    E: Event,
    F: Fn(E) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), EventBusError>> + Send + 'static,
  {
    let event_type = TypeId::of::<E>();

    if self.handlers.contains_key(&event_type) {
      return Err(EventBusError::HandlerAlreadyRegistered {
        event_name: E::NAME,
      });
    }

    self.handlers.insert(
      event_type,
      Box::new(move |event| {
        let event = event
          .downcast::<E>()
          .expect("event type must match its registered handler");

        Box::pin(handler(*event))
      }),
    );

    Ok(())
  }

  /// Runs the local handler and returns only after it has completed.
  pub async fn publish<E>(&self, event: E) -> Result<(), EventBusError>
  where
    E: Event,
  {
    let handler = self.handlers
      .get(&TypeId::of::<E>())
      .ok_or(EventBusError::HandlerNotRegistered {
        event_name: E::NAME,
      })?;

    handler(Box::new(event)).await
  }
}
