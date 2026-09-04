use std::{future::Future, pin::Pin};

use crate::channel::attachment::{AttachCommand, DetachCommand};
use crate::channel::presence::snapshot::PresenceSnapshot;
use crate::channel::presence::transition::CommittedTransition;
use crate::connection::DisconnectConnectionCommand;
use crate::{AggregatedOccupancyShard, ChannelKey, OccupancyShardFlushResult, PresenceAttachOutcome, PresenceMutationReceipt, PresenceStoreError};
use crate::channel::presence::command::PresenceBatchCommand;

pub type PresenceStoreFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, PresenceStoreError>> + Send + 'a>>;

pub trait PresenceStore: Send + Sync {
  fn attach_and_snapshot(&self, command: AttachCommand) -> PresenceStoreFuture<'_, PresenceAttachOutcome>;

  fn apply_presence(&self, command: PresenceBatchCommand) -> PresenceStoreFuture<'_, PresenceMutationReceipt>;

  fn detach(&self, command: DetachCommand) -> PresenceStoreFuture<'_, CommittedTransition>;

  fn disconnect(&self, command: DisconnectConnectionCommand) -> PresenceStoreFuture<'_, Vec<CommittedTransition>>;

  fn snapshot(&self, channel: ChannelKey) -> PresenceStoreFuture<'_, PresenceSnapshot>;

  fn flush_occupancy_shard(&self, shard: AggregatedOccupancyShard) -> PresenceStoreFuture<'_, OccupancyShardFlushResult>;
}
