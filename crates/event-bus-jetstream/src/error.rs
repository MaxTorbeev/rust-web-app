use thiserror::Error;

/// Failure to construct a valid JetStream publisher configuration.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum JetStreamPublisherConfigError {
  #[error("missing required environment variable `{variable}`")]
  MissingEnvironmentVariable {
    variable: &'static str,

    #[source]
    source: std::env::VarError,
  },

  #[error("invalid value {value:?} for `{component}`: {reason}")]
  InvalidNamespaceComponent {
    component: &'static str,
    value: String,
    reason: &'static str,
  },
}

#[derive(Debug, Error)]
pub(crate) enum EventSubjectError {
  #[error("event name `{event_name}` is not a valid NATS subject suffix")]
  InvalidEventName { event_name: String },

  #[error("LocalOnly events cannot be published to JetStream")]
  UnsupportedDeliveryClass,
}
