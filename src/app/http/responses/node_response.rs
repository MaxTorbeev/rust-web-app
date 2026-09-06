use serde::Serialize;

use support::{DeploymentSlot, NodeIdentity};

/// Блок `node` health-ответов: стабильный `APP_NODE_ID` и deployment slot.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NodeResponse {
  id: String,
  slot: DeploymentSlot,
}

impl From<&NodeIdentity> for NodeResponse {
  fn from(node: &NodeIdentity) -> Self {
    Self {
      id: node.id().as_str().to_owned(),
      slot: node.slot(),
    }
  }
}
