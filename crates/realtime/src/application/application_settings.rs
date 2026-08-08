use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct ApplicationSettings {
  pub max_message_size: u64,
  pub max_inbound_rate: u64,
  pub max_outbound_rate: u64,
  pub max_frame_size: u64,
  pub connection_state_ttl: u64,
  pub max_idle_interval: u64,
}

impl Default for ApplicationSettings {
  fn default() -> Self {
    Self {
      max_message_size: 262144,
      max_inbound_rate: 50,
      max_outbound_rate: 50,
      max_frame_size: 1468006,
      connection_state_ttl: 120_000,
      max_idle_interval: 15_000,
    }
  }
}