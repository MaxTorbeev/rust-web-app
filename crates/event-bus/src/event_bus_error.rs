use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum EventBusError {
  Driver(tokio_events::Error),
}

impl From<tokio_events::Error> for EventBusError {
  fn from(e: tokio_events::Error) -> Self {
    Self::Driver(e)
  }
}

impl Display for EventBusError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Driver(e) => write!(f, "{:?}", e),
    }
  }
}

impl std::error::Error for EventBusError {}