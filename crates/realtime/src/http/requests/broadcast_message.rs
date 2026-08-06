use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct BroadcastMessage {
  pub name: Option<String>,
  pub data: serde_json::Value,
}