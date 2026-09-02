use std::{collections::BTreeSet, future::Future, pin::Pin};

use serde_json::Value;
use uuid::Uuid;

use crate::{ApplicationId, AttachCommand, ChannelKey, ChannelMode, ConnectionId, PresenceAction, PresenceBatchCommand};
use thiserror::Error;

pub type PresenceStoreFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, PresenceStoreError>> + Send + 'a>>;

pub type NodeId = String;
pub type BootGeneration = String;

#[derive(Debug, Error)]
pub enum PresenceStoreError {
  #[error("invalid request: {message}")]
  InvalidRequest { message: String },
  #[error("entry not found: {message}")]
  NotFound { message: String },
  #[error("duplicate operation: {message}")]
  Duplicate { message: String },
  #[error("operation conflict: {message}")]
  Conflict { message: String },
  #[error("protocol conflict")]
  ProtocolConflict,
  #[error("internal store error: {message}")]
  Internal { message: String },
}

#[derive(Debug, Clone)]
pub struct PresenceOwner {
  pub node_id: NodeId,
  pub boot_generation: BootGeneration,
}

#[derive(Debug, Clone)]
pub struct PresenceActor {
  pub connection_id: ConnectionId,
  pub owner: PresenceOwner,
  /// All client ids allowed for this connection. Presence identity remains
  /// `(connection_id, client_id)`.
  pub authorized_client_ids: Vec<String>,
}
#[derive(Debug, Clone)]
pub struct PresenceBatchItem {
  pub action: PresenceAction,
  pub client_id: String,
  pub message_id: String,
  pub data: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct DetachCommand {
  pub channel: ChannelKey,
  pub actor: PresenceActor,

  /// Deterministic normalized payload hash for replay comparison.
  pub normalized_request_hash: String,

  /// Stable operation identifier for idempotent storage outcome.
  pub operation_id: Uuid,

  /// Server timestamp in ms.
  pub request_time_ms: u64,
}

#[derive(Debug, Clone)]
pub struct DisconnectCommand {
  pub actor: PresenceActor,

  /// Stable operation identifier for idempotent storage outcome.
  pub operation_id: Uuid,

  /// Deterministic normalized request hash for replay comparison.
  pub normalized_request_hash: String,

  /// Server timestamp in ms.
  pub request_time_ms: u64,
}

#[derive(Debug, Clone)]
pub struct PresenceMember {
  pub connection_id: ConnectionId,
  pub client_id: String,
  pub owner: PresenceOwner,
  pub data: Option<Value>,
  pub last_message_id: String,
  pub presence_revision: u64,
  pub updated_at_ms: u64,
}

#[derive(Debug, Clone)]
pub struct PresenceSnapshot {
  pub members: Vec<PresenceMember>,
  pub presence_revision: u64,
  pub occupancy: OccupancyMetrics,
}

#[derive(Debug, Clone)]
pub struct PresenceDelta {
  pub action: PresenceAction,
  pub connection_id: ConnectionId,
  pub client_id: String,
  pub data: Option<Value>,
  pub message_id: String,
  pub timestamp_ms: u64,
}

#[derive(Debug, Clone)]
pub struct CommittedTransition {
  pub presence_revision: Option<u64>,
  pub occupancy_version: u64,
  pub event_id: Option<Uuid>,
  pub duplicate: bool,
}

#[derive(Debug, Clone)]
pub struct AttachResult {
  pub snapshot: PresenceSnapshot,
  pub transition: CommittedTransition,
  pub effective_modes: Vec<ChannelMode>,
  pub effective_occupancy: Option<OccupancySubscription>,
}

/// Absolute per-owner contribution for attachments represented only in
/// aggregated Occupancy state.
#[derive(Debug, Clone)]
pub struct AggregatedOccupancyShard {
  pub owner: PresenceOwner,
  pub channel: ChannelKey,
  pub version: u64,
  pub connections: u64,
  pub subscribers: u64,
  pub presence_subscribers: u64,
  pub lease_deadline_ms: u64,
}

#[derive(Debug, Clone)]
pub struct OccupancyShardFlushResult {
  pub occupancy_version: u64,
  pub global_zero_boundary: bool,
  pub snapshot: OccupancyMetrics,
}

#[derive(Debug, Clone)]
pub struct OccupancyMetrics {
  pub connections: u64,
  pub publishers: u64,
  pub subscribers: u64,
  pub presence_connections: u64,
  pub presence_subscribers: u64,
  pub presence_members: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum OccupancyCategory {
  Connections,
  Publishers,
  Subscribers,
  PresenceConnections,
  PresenceSubscribers,
  PresenceMembers,
}

#[derive(Debug, Clone)]
pub struct OccupancyChange {
  pub metrics: OccupancyMetrics,
  pub changed_categories: BTreeSet<OccupancyCategory>,
  pub zero_boundary_categories: BTreeSet<OccupancyCategory>,
}

#[derive(Debug, Clone)]
pub enum OccupancySubscription {
  Metrics,
  Category(OccupancyCategory),
  Categories(Vec<OccupancyCategory>),
}

#[derive(Debug, Clone)]
pub struct Attachment {
  pub connection_id: ConnectionId,
  pub owner: PresenceOwner,
  pub effective_modes: Vec<ChannelMode>,
  pub occupancy: Option<OccupancySubscription>,
}

pub trait PresenceStore: Send + Sync {
  fn attach_and_snapshot(&self, command: AttachCommand) -> PresenceStoreFuture<'_, AttachResult>;

  fn apply_presence(&self, command: PresenceBatchCommand) -> PresenceStoreFuture<'_, CommittedTransition>;

  fn detach(&self, command: DetachCommand) -> PresenceStoreFuture<'_, CommittedTransition>;

  fn disconnect(&self, command: DisconnectCommand) -> PresenceStoreFuture<'_, Vec<CommittedTransition>>;

  fn snapshot(&self, channel: ChannelKey) -> PresenceStoreFuture<'_, PresenceSnapshot>;

  fn flush_occupancy_shard(&self, shard: AggregatedOccupancyShard) -> PresenceStoreFuture<'_, OccupancyShardFlushResult>;
}
