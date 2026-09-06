use serde::Serialize;

use support::NodeIdentity;

use crate::app::health::HealthCheck;
use crate::app::version::AppVersion;

use super::{HEALTH_SCHEMA_VERSION, NodeResponse, ReleaseResponse};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LiveHealthResponse {
  schema_version: u8,
  status: LiveStatus,
  node: NodeResponse,
  release: ReleaseResponse,
}

impl LiveHealthResponse {
  fn new(version: AppVersion, node: &NodeIdentity) -> Self {
    Self {
      schema_version: HEALTH_SCHEMA_VERSION,
      status: LiveStatus::Alive,
      node: NodeResponse::from(node),
      release: ReleaseResponse::from(version),
    }
  }
}

impl From<&HealthCheck> for LiveHealthResponse {
  /// Liveness не выполняет проверок зависимостей: берёт только метаданные.
  fn from(health: &HealthCheck) -> Self {
    Self::new(health.version(), health.node())
  }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum LiveStatus {
  Alive,
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;
  use support::{DeploymentSlot, NodeId};

  #[test]
  fn live_response_carries_node_identity() {
    let node = NodeIdentity::new(
      NodeId::try_new("realtime-test").expect("test node id must be valid"),
      DeploymentSlot::Green,
    );

    let encoded = serde_json::to_value(LiveHealthResponse::new(AppVersion::CURRENT, &node)).unwrap();

    assert_eq!(encoded["schemaVersion"], json!(HEALTH_SCHEMA_VERSION));
    assert_eq!(encoded["status"], json!("alive"));
    assert_eq!(encoded["node"], json!({ "id": "realtime-test", "slot": "green" }));
    assert_eq!(encoded["release"]["version"], json!(AppVersion::CURRENT.version()));
  }
}
