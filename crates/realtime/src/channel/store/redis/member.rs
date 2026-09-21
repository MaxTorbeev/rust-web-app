use serde::{Deserialize, Serialize};
use support::NodeInstance;

use crate::ConnectionId;

/// Подготовленный Rust payload участника; revision и timestamp хранит Redis отдельно.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MemberPayload {
  pub connection_id: ConnectionId,
  pub client_id: String,
  pub node_instance: NodeInstance,
  pub data_json: Option<String>,
  pub last_message_id: String,
}
