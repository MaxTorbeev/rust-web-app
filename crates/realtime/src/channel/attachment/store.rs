use std::{future::Future, pin::Pin};

use crate::channel::attachment::{AttachCommand, DetachCommand};
use crate::connection::DisconnectConnectionCommand;
use crate::{ChannelAttachOutcome, ChannelStateStoreError, CommittedChannelTransition};

pub type AttachmentStoreFuture<'a, T> =
  Pin<Box<dyn Future<Output = Result<T, ChannelStateStoreError>> + Send + 'a>>;

/// Хранилище состояния соединений, работающих с каналами.
pub trait AttachmentStore: Send + Sync {
  fn attach_and_snapshot(
    &self,
    command: AttachCommand,
  ) -> AttachmentStoreFuture<'_, ChannelAttachOutcome>;

  fn detach(&self, command: DetachCommand) -> AttachmentStoreFuture<'_, CommittedChannelTransition>;

  fn disconnect(
    &self,
    command: DisconnectConnectionCommand,
  ) -> AttachmentStoreFuture<'_, Vec<CommittedChannelTransition>>;
}
