use serde::{Deserialize, Serialize};
use support::fresh_uuid;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ConnectionId(String);

impl ConnectionId {
  pub fn generate() -> Self {
    Self(fresh_uuid().to_string())
  }

  pub fn as_str(&self) -> &str {
    &self.0
  }
}
