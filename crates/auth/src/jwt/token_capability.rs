use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TokenCapability {
  resources: HashMap<String, HashSet<String>>,
}

impl TokenCapability {
  pub fn resources(&self) -> &HashMap<String, HashSet<String>> {
    &self.resources
  }

  /// Проверяет, разрешена ли операция над ресурсом.
  ///
  /// Ресурс сопоставляется с точным именем, namespace wildcard `ns:*`
  /// (любой канал `ns:...`) и общим `*`. Операция `*` разрешает все операции.
  pub fn allows(&self, resource: &str, operation: &str) -> bool {
    self
      .resources
      .iter()
      .any(|(pattern, operations)| {
        Self::resource_matches(pattern, resource)
          && (operations.contains(operation) || operations.contains("*"))
      })
  }

  fn resource_matches(pattern: &str, resource: &str) -> bool {
    if pattern == "*" {
      return true;
    }

    if let Some(namespace) = pattern.strip_suffix(":*") {
      return resource
        .strip_prefix(namespace)
        .is_some_and(|rest| rest.starts_with(':'));
    }

    pattern == resource
  }
}

impl FromStr for TokenCapability {
  type Err = serde_json::Error;

  /// Convert from json:
  /// {
  ///   "private:*": ["subscribe", "publish", "presence"],
  ///   "notifications": ["subscribe"]
  /// }
  fn from_str(value: &str) -> Result<Self, Self::Err> {
    serde_json::from_str(value)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_capability() {
    let capability = r#"{
      "private:*": ["subscribe", "publish", "presence"],
      "notifications": ["subscribe"]
    }"#
      .parse::<TokenCapability>()
      .unwrap();

    let operations = capability.resources().get("private:*").unwrap();

    assert!(operations.contains("subscribe"));
    assert!(operations.contains("publish"));
    assert!(operations.contains("presence"));
  }

  #[test]
  fn allows_resolves_wildcards() {
    let capability = r#"{
      "private:*": ["subscribe", "presence"],
      "notifications": ["subscribe"],
      "admin": ["*"]
    }"#
      .parse::<TokenCapability>()
      .unwrap();

    assert!(capability.allows("private:room", "presence"));
    assert!(!capability.allows("private:room", "publish"));
    assert!(!capability.allows("private", "subscribe"));
    assert!(!capability.allows("privateroom", "subscribe"));
    assert!(capability.allows("notifications", "subscribe"));
    assert!(!capability.allows("notifications:x", "subscribe"));
    assert!(capability.allows("admin", "channel-metadata"));
    assert!(!capability.allows("other", "subscribe"));

    let global = r#"{"*": ["subscribe"]}"#.parse::<TokenCapability>().unwrap();
    assert!(global.allows("anything:at:all", "subscribe"));
    assert!(!global.allows("anything:at:all", "publish"));
  }
}
