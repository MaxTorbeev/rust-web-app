use crate::app::providers::EventBusConfigMapperError;
use event_bus::HandlerRegistrationError;
use event_bus_jetstream::JetStreamConsumerError;
use nats_client::{ConnectError, StreamSetupError, SubscribeError};
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum EventBusProviderError {
  #[error("failed to load Event Bus configuration: {0}")]
  ConfigLoad(#[from] confique::Error),

  #[error("invalid Event Bus configuration: {0}")]
  ConfigMap(#[from] EventBusConfigMapperError),

  #[error("failed to register Event Bus handlers: {0}")]
  HandlerRegistration(#[from] HandlerRegistrationError),

  #[error("failed to connect to NATS: {0}")]
  Connect(#[from] ConnectError),

  #[error("failed to configure JetStream stream: {0}")]
  StreamSetup(#[from] StreamSetupError),

  #[error("failed to subscribe to JetStream: {0}")]
  Subscribe(#[from] SubscribeError),
}

#[derive(Debug, Error)]
pub enum EventBusRuntimeError {
  #[error("incoming JetStream consumer stopped")]
  Consumer(#[source] JetStreamConsumerError),
}
