use serde::Deserialize;

#[derive(Deserialize)]
pub struct BroadcastMessage {
  pub name: Option<String>,
  pub data: serde_json::Value,
}