use serde::{Deserialize, Serialize};
use crate::PresenceAction;

#[derive(Serialize, Deserialize)]
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
  pub timestamp: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub data: Option<serde_json::Value>,
}