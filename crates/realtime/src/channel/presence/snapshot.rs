use crate::{OccupancyMetrics, PresenceMember};

#[derive(Debug, Clone)]
pub struct PresenceSnapshot {
  pub members: Vec<PresenceMember>,
  pub presence_revision: u64,
  pub occupancy: OccupancyMetrics,
}