use serde::{Deserialize, Serialize};
use crate::OccupancyCategory;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OccupancyMetrics {
  pub connections: u64,
  pub publishers: u64,
  pub subscribers: u64,
  pub presence_connections: u64,
  pub presence_subscribers: u64,
  pub presence_members: u64,
}

impl OccupancyMetrics {
  pub fn entries(&self) -> impl Iterator<Item = (OccupancyCategory, u64)> {
    [
      (OccupancyCategory::Connections, self.connections),
      (OccupancyCategory::Publishers, self.publishers),
      (OccupancyCategory::Subscribers, self.subscribers),
      (OccupancyCategory::PresenceConnections, self.presence_connections),
      (OccupancyCategory::PresenceSubscribers, self.presence_subscribers),
      (OccupancyCategory::PresenceMembers, self.presence_members),
    ].into_iter()
  }
}