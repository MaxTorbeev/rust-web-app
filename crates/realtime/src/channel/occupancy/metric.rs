use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OccupancyMetrics {
  pub connections: u64,
  pub publishers: u64,
  pub subscribers: u64,
  pub presence_connections: u64,
  pub presence_subscribers: u64,
  pub presence_members: u64,
}
