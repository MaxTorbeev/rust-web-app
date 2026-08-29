use support::app::{APP_NAMESPACE_SEPARATOR, AppNamespace};

/// Shared JetStream subject topology for one application namespace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JetStreamSubjectConfig {
  prefix: String,
}

impl JetStreamSubjectConfig {
  /// Builds subject topology from an already validated application namespace.
  pub fn new(namespace: &AppNamespace) -> Self {
    Self {
      prefix: namespace.as_str().to_owned(),
    }
  }

  /// Subject filter for events that must be applied independently on each node.
  pub fn all_nodes_subject_filter(&self) -> String {
    format!(
      "{}{APP_NAMESPACE_SEPARATOR}all{APP_NAMESPACE_SEPARATOR}>",
      self.prefix
    )
  }

  pub(crate) fn prefix(&self) -> &str {
    &self.prefix
  }
}
