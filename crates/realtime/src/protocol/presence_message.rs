use serde::{Deserialize, Serialize};
use crate::PresenceAction;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresenceMessage {
  pub action: PresenceAction,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub client_id: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub connection_id: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub timestamp: Option<u64>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub data: Option<serde_json::Value>,
}