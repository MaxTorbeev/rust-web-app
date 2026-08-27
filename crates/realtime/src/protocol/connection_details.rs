use crate::Connection;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDetails {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub client_id: Option<String>,
  pub connection_key: String,
  pub max_message_size: u64,
  pub max_inbound_rate: u64,
  pub max_outbound_rate: u64,
  pub max_frame_size: u64,
  pub connection_state_ttl: u64,
  pub max_idle_interval: u64,
}

impl From<&Connection> for ConnectionDetails {
  fn from(connection: &Connection) -> Self {
    let settings = connection.settings();

    Self {
      client_id: connection.client_id().map(|s| s.to_string()),
      connection_key: connection.connection_key().to_owned(),
      max_message_size: settings.max_message_size,
      max_inbound_rate: settings.max_inbound_rate,
      max_outbound_rate: settings.max_outbound_rate,
      max_frame_size: settings.max_frame_size,
      connection_state_ttl: settings.connection_state_ttl,
      max_idle_interval: settings.max_idle_interval,
    }
  }
}
