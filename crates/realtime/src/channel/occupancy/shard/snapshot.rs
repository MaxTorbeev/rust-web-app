use support::NodeInstance;
use crate::ChannelKey;

/// Absolute per-owner contribution for attachments represented only in
/// aggregated Occupancy state.
#[derive(Debug, Clone)]
pub struct OccupancyShardSnapshot {
  pub owner: NodeInstance,
  pub channel: ChannelKey,
  pub version: u64,
  pub connections: u64,
  pub subscribers: u64,
  pub presence_subscribers: u64,
  pub lease_deadline_ms: u64,
}