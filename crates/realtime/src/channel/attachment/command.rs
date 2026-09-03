use uuid::Uuid;
use support::timestamp::Timestamp;
use crate::{ChannelKey, ChannelMode, OccupancySubscription};
use crate::connection::ConnectionActor;

/// Команда на запрос соединения.
#[derive(Debug, Clone)]
pub struct AttachCommand {
  pub channel: ChannelKey,
  pub actor: ConnectionActor,

  /// Attach retry key: (application_id, connection_id, msg_serial),
  /// passed separately through protocol handler.
  pub msg_serial: Option<u64>,

  /// Stable operation identifier for idempotent storage outcome.
  pub operation_id: Uuid,

  /// Deterministic normalized payload hash for exact payload replay checks.
  pub normalized_request_hash: String,

  /// Server timestamp in ms.
  pub request_time_ms: u64,

  /// Effective (server calculated) modes and requested occupancy subscription.
  pub requested_modes: Vec<ChannelMode>,
  pub occupancy: Option<OccupancySubscription>,
}

#[derive(Debug, Clone)]
pub struct DetachCommand {
  pub channel: ChannelKey,
  pub actor: ConnectionActor,
  pub operation_id: Uuid,
  pub normalized_request_hash: String,
  pub request_time: Timestamp,
}
