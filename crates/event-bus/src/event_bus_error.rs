use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum EventBusError {
  Driver(tokio_events::Error),
  Encode(serde_json::Error),
  Decode(serde_json::Error),
  EventTypeMismatch {
    expected: String,
    actual: String,
  },
  EventVersionMismatch {
    event_name: String,
    expected: u16,
    actual: u16,
  },
  Publisher(Box<dyn std::error::Error + Send + Sync>),
}

impl EventBusError {
  pub fn publisher(error: impl std::error::Error + Send + Sync + 'static) -> Self {
    Self::Publisher(Box::new(error))
  }
}

impl From<tokio_events::Error> for EventBusError {
  fn from(e: tokio_events::Error) -> Self {
    Self::Driver(e)
  }
}

impl Display for EventBusError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Driver(error) => write!(f, "{error}"),
      Self::Encode(error) => write!(f, "failed to encode event: {error}"),
      Self::Decode(error) => write!(f, "failed to decode event: {error}"),
      Self::EventTypeMismatch { expected, actual } => {
        write!(f, "event type mismatch: expected {expected}, got {actual}")
      },
      Self::EventVersionMismatch {
        event_name,
        expected,
        actual,
      } => write!(
        f,
        "event version mismatch for {event_name}: expected {expected}, got {actual}"
      ),
      Self::Publisher(error) => write!(f, "failed to publish event: {error}"),
    }
  }
}

impl std::error::Error for EventBusError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      Self::Driver(error) => Some(error),
      Self::Encode(error) | Self::Decode(error) => Some(error),
      Self::Publisher(error) => Some(error.as_ref()),
      Self::EventTypeMismatch { .. } | Self::EventVersionMismatch { .. } => None,
    }
  }
}
