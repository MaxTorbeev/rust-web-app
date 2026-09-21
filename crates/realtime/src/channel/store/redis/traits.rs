use crate::{
  AttachCommand, AttachmentStore, AttachmentStoreFuture, ChannelAttachOutcome, ChannelKey,
  CommittedChannelTransition, DetachCommand, DisconnectConnectionCommand, PresenceBatchCommand,
  PresenceMutationReceipt, PresenceSnapshot, PresenceStore, PresenceStoreFuture,
};

use super::RedisChannelStore;

impl AttachmentStore for RedisChannelStore {
  fn attach_and_snapshot(
    &self,
    command: AttachCommand,
  ) -> AttachmentStoreFuture<'_, ChannelAttachOutcome> {
    Box::pin(RedisChannelStore::attach_and_snapshot(self, command))
  }

  fn detach(
    &self,
    command: DetachCommand,
  ) -> AttachmentStoreFuture<'_, CommittedChannelTransition> {
    Box::pin(RedisChannelStore::detach(self, command))
  }

  fn disconnect(
    &self,
    command: DisconnectConnectionCommand,
  ) -> AttachmentStoreFuture<'_, Vec<CommittedChannelTransition>> {
    Box::pin(RedisChannelStore::disconnect(self, command))
  }
}

impl PresenceStore for RedisChannelStore {
  fn apply_presence(
    &self,
    command: PresenceBatchCommand,
  ) -> PresenceStoreFuture<'_, PresenceMutationReceipt> {
    Box::pin(RedisChannelStore::apply_presence(self, command))
  }

  fn snapshot(&self, channel: ChannelKey) -> PresenceStoreFuture<'_, PresenceSnapshot> {
    Box::pin(RedisChannelStore::snapshot(self, channel))
  }
}
