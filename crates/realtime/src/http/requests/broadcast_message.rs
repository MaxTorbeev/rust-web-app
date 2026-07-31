use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct BroadcastMessage {
  pub name: Option<String>,
  pub data: serde_json::Value,
}