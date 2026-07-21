use std::fmt::Debug;
use std::future::Future;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio_events::EventBusBuilder;
use crate::event_bus_error::EventBusError;
use crate::{ListenerHandle};

pub trait Event: Clone
+ Debug
+ Send
+ Sync
+ Serialize
+ DeserializeOwned
+ 'static {
  const NAME: &'static str;
}

#[derive(Clone, Debug)]
struct TokioEventEnvelope<E: Event> {
  event: E,
}

pub struct EventBus {
  inner: tokio_events::bus::EventBus,
}

impl<E> tokio_events::Event for TokioEventEnvelope<E>
where
  E: Event,
{
  fn event_type() -> &'static str {
    E::NAME
  }

  fn serialize_event(&self) -> tokio_events::Result<Vec<u8>> {
    serde_json::to_vec(&self.event)
      .map_err(|e| tokio_events::Error::SerializationError(e.to_string()))
  }

  fn deserialize_event(bytes: &[u8]) -> tokio_events::Result<Self> {
    let event: E = serde_json::from_slice(&bytes)
      .map_err(|e| tokio_events::Error::SerializationError(e.to_string()))?;

    Ok(Self { event })
  }
}

impl EventBus {
  pub async fn new() -> Result<Self, EventBusError> {
    let inner = EventBusBuilder::new()
      .build()
      .await?;

    Ok(Self { inner })
  }

  pub async fn publish<E>(&self, event: E) -> Result<(), EventBusError> where E: Event {
    self.inner.publish(TokioEventEnvelope {event}).await?;

    Ok(())
  }

  pub async fn emit<E>(&self, event: E) where E: Event {
    if let Err(e) = self.publish(event).await {
      tracing::error!(event = E::NAME, %e, "Failed to publish event");
    }
  }

  pub async fn listen<E, F, Fut>(&self, handler: F) -> Result<(), EventBusError>
  where E: Event,
        F: Fn(E) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static
  {
    let handle = self.subscribe(handler).await?;

    handle.detach();

    Ok(())
  }

  pub async fn subscribe<E, F, Fut>(&self, handler: F) -> Result<ListenerHandle, EventBusError>
  where
    E: Event,
    F: Fn(E) -> Fut + Send + Sync + 'static,
    Fut: Future<Output=()> + Send + 'static,
  {
    let inner = self.inner.subscribe(move |envelope: TokioEventEnvelope<E>| { handler(envelope.event) })
      .await?;

    Ok(ListenerHandle::new(inner))
  }
}

