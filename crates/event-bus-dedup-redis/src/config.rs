use support::app::{APP_NAMESPACE_SEPARATOR, AppNamespace};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RedisDedupStoreConfig {
  key_prefix: String,
}

impl RedisDedupStoreConfig {
  pub fn new(namespace: &AppNamespace) -> Self {
    Self {
      key_prefix: format!("{namespace}{APP_NAMESPACE_SEPARATOR}dedup"),
    }
  }

  pub(crate) fn key_prefix(&self) -> &str {
    &self.key_prefix
  }
}
