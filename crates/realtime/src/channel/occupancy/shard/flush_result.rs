use crate::OccupancyMetrics;

#[derive(Debug, Clone)]
pub struct OccupancyShardFlushResult {
  pub occupancy_version: u64,
  pub global_zero_boundary: bool,
  pub snapshot: OccupancyMetrics,
}
