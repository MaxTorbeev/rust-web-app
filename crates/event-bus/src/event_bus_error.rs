use thiserror::Error;

#[derive(Debug, Error)]
pub enum EventBusError {
  #[error("failed to encode event: {0}")]
  Encode(#[source] serde_json::Error),

  #[error("failed to decode event: {0}")]
  Decode(#[source] serde_json::Error),

  #[error("handler for event {event_name} is already registered")]
  HandlerAlreadyRegistered {
    event_name: &'static str,
  },

  #[error("handler for event {event_name} is not registered")]
  HandlerNotRegistered {
    event_name: &'static str,
  },

  #[error("event type mismatch: expected {expected}, got {actual}")]
  EventTypeMismatch {
    expected: String,
    actual: String,
  },

  #[error(
    "event version mismatch for {event_name}: expected {expected}, got {actual}"
  )]
  EventVersionMismatch {
    event_name: String,
    expected: u16,
    actual: u16,
  },

  #[error("failed to publish event: {0}")]
  Publisher(
    #[source]
    Box<dyn std::error::Error + Send + Sync>,
  ),

  #[error("handler for event {event_name} failed: {source}")]
  Handler {
    event_name: &'static str,

    #[source]
    source: Box<dyn std::error::Error + Send + Sync>,
  },
}

impl EventBusError {
  pub fn publisher(error: impl std::error::Error + Send + Sync + 'static) -> Self {
    Self::Publisher(Box::new(error))
  }

  pub fn handler(
    event_name: &'static str,
    error: impl std::error::Error + Send + Sync + 'static,
  ) -> Self {
    Self::Handler {
      event_name,
      source: Box::new(error),
    }
  }
}
