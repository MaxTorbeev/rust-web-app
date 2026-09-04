use crate::ConnectionId;
use serde_json::Value;
use support::NodeInstance;

#[derive(Debug, Clone)]
pub struct PresenceMember {
  pub connection_id: ConnectionId,
  pub client_id: String,
  pub node_instance: NodeInstance,
  pub data: Option<Value>,
  pub last_message_id: String,
  pub presence_revision: u64,
  pub updated_at_ms: u64,
}
