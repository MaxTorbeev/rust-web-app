use support::app::AppNamespace;

/// Configuration required to map events to JetStream subjects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JetStreamPublisherConfig {
  subject_prefix: String,
}

impl JetStreamPublisherConfig {
  /// Builds a configuration from an already validated application namespace.
  pub fn new(namespace: &AppNamespace) -> Self {
    Self {
      subject_prefix: namespace.as_str().to_owned(),
    }
  }

  pub(crate) fn subject_prefix(&self) -> &str {
    &self.subject_prefix
  }
}
