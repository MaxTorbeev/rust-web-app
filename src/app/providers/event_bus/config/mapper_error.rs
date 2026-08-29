use std::fmt::Display;

use event_bus::IncomingEventProcessorConfigError;
use event_bus_jetstream::JetStreamIncomingConsumerConfigError;
use nats_client::{ConsumerConfigError, NatsConfigError, StreamConfigError, StreamLimitsError};
use support::app::AppNamespaceError;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum EventBusConfigMapperError {
  #[error("invalid NATS connection configuration: {0}")]
  NatsConfig(#[from] NatsConfigError),

  #[error("invalid Event Bus configuration: {0}")]
  InvalidConfig(String),
}

impl EventBusConfigMapperError {
  pub(super) fn invalid(error: impl Display) -> Self {
    Self::InvalidConfig(error.to_string())
  }
}

impl From<AppNamespaceError> for EventBusConfigMapperError {
  fn from(error: AppNamespaceError) -> Self {
    Self::invalid(error)
  }
}

impl From<StreamLimitsError> for EventBusConfigMapperError {
  fn from(error: StreamLimitsError) -> Self {
    Self::invalid(error)
  }
}

impl From<StreamConfigError> for EventBusConfigMapperError {
  fn from(error: StreamConfigError) -> Self {
    Self::invalid(error)
  }
}

impl From<ConsumerConfigError> for EventBusConfigMapperError {
  fn from(error: ConsumerConfigError) -> Self {
    Self::invalid(error)
  }
}

impl From<IncomingEventProcessorConfigError> for EventBusConfigMapperError {
  fn from(error: IncomingEventProcessorConfigError) -> Self {
    Self::invalid(error)
  }
}

impl From<JetStreamIncomingConsumerConfigError> for EventBusConfigMapperError {
  fn from(error: JetStreamIncomingConsumerConfigError) -> Self {
    Self::invalid(error)
  }
}
