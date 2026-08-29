use event_bus_jetstream::JetStreamConsumerError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EventBusProviderError {

}

#[derive(Debug, Error)]
pub enum EventBusRuntimeError {
  #[error("incoming JetStream consumer stopped")]
  Consumer(#[source] JetStreamConsumerError),
}