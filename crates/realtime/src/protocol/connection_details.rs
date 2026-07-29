use serde::{Deserialize, Serialize};
use crate::Connection;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDetails {
  #[serde(skip_serializing_if="Option::is_none")]
  pub client_id: Option<String>,
  pub connection_key: String,
  pub max_message_size: u64,
  pub max_inbound_rate: u64,
  pub max_outbound_rate: u64,
  pub max_frame_size: u64,
  pub connection_state_ttl: u64,
  pub max_idle_interval: u64,
}

impl ConnectionDetails {
  pub fn new(connection: &Connection) -> Self {
    Self {
      client_id: connection.client_id().map(str::to_owned),
      connection_key: uuid::Uuid::new_v4().to_string(),
      max_message_size: 262144,
      max_inbound_rate: 50,
      max_outbound_rate: 50,
      max_frame_size: 1468006,
      connection_state_ttl: 120000,
      max_idle_interval: 15000,
    }
  }
}