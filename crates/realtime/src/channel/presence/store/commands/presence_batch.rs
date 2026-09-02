use uuid::Uuid;
use crate::ChannelKey;

#[derive(Debug, Clone)]
pub struct PresenceBatchCommand {
  pub channel: ChannelKey,
  pub actor: PresenceActor,
  pub items: Vec<PresenceBatchItem>,

  /// Retry key for protocol-level deduplication.
  pub msg_serial: Option<u64>,

  /// Stable operation identifier for idempotent storage outcome.
  pub operation_id: Uuid,

  /// Deterministic normalized payload hash for replay comparison.
  pub normalized_request_hash: String,

  /// Server timestamp in ms.
  pub request_time_ms: u64,

  /// Effective (server calculated) modes and requested occupancy subscription.
  pub effective_modes: Vec<ChannelMode>,
  pub occupancy: Option<OccupancySubscription>,
}