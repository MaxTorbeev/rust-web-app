use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
  pub name: Option<String>,
  pub data: serde_json::Value,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub client_id: Option<String>,
}
