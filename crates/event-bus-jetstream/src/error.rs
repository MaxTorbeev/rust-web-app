use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum EventSubjectError {
  #[error("event name `{event_name}` is not a valid NATS subject suffix")]
  InvalidEventName { event_name: String },

  #[error("LocalOnly events cannot be published to JetStream")]
  UnsupportedDeliveryClass,
}
