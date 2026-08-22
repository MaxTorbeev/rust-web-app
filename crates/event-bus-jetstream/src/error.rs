use thiserror::Error;

#[derive(Debug, Error)]
pub enum JetStreamPublisherError {
  #[error("missing required env var `{variable}`")]
  MissingEnv {
    variable: &'static str,
    #[source]
    source: std::env::VarError,
  },
  #[error("invalid value for env var `{variable}`: {reason}")]
  InvalidEnv {
    variable: &'static str,
    value: String,
    reason: String,
  },
  #[error("LocalOnly events cannot be published to JetStream")]
  UnsupportedDeliveryClass,
}
