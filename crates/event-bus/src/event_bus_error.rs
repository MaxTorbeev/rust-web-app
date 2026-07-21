#[derive(Debug)]
pub enum EventBusError {
  Driver(tokio_events::Error),
}

impl From<tokio_events::Error> for EventBusError {
  fn from(e: tokio_events::Error) -> Self {
    Self::Driver(e)
  }
}