use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct TokenCapability {
  resources: HashMap<String, HashSet<String>>,
}

impl TokenCapability {
  pub fn resources(&self) -> &HashMap<String, HashSet<String>> {
    &self.resources
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

    let operations = capability
      .resources()
      .get("private:*")
      .unwrap();

    assert!(operations.contains("subscribe"));
    assert!(operations.contains("publish"));
    assert!(operations.contains("presence"));
  }
}