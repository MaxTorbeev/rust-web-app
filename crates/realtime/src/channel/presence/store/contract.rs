use std::{future::Future, pin::Pin};

use crate::channel::presence::command::PresenceBatchCommand;
use crate::channel::presence::snapshot::PresenceSnapshot;
use crate::{ChannelKey, ChannelStateStoreError, PresenceMutationReceipt};

pub type PresenceStoreFuture<'a, T> =
  Pin<Box<dyn Future<Output = Result<T, ChannelStateStoreError>> + Send + 'a>>;

pub trait PresenceStore: Send + Sync {
  fn apply_presence(
    &self,
    command: PresenceBatchCommand,
  ) -> PresenceStoreFuture<'_, PresenceMutationReceipt>;

  fn snapshot(&self, channel: ChannelKey) -> PresenceStoreFuture<'_, PresenceSnapshot>;
}
