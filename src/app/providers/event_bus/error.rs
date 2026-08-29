use event_bus_jetstream::JetStreamConsumerError;
use thiserror::Error;
use event_bus::HandlerRegistrationError;

#[derive(Debug, Error)]
pub enum EventBusProviderError {
  #[error("failed to register event bus handlers")]
  HandlerRegistration(#[from] HandlerRegistrationError),
}

#[derive(Debug, Error)]
pub enum EventBusRuntimeError {
  #[error("incoming JetStream consumer stopped")]
  Consumer(#[source] JetStreamConsumerError),
}