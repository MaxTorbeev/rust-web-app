use std::{future::Future, pin::Pin};

use crate::channel::attachment::{AttachCommand, DetachCommand};
use crate::connection::DisconnectConnectionCommand;
use crate::{CommittedTransition, PresenceAttachOutcome, PresenceStoreError};

pub type AttachmentStoreFuture<'a, T> =
  Pin<Box<dyn Future<Output = Result<T, PresenceStoreError>> + Send + 'a>>;

/// Хранилище состояния соединений, работающих с каналами.
pub trait AttachmentStore: Send + Sync {
  fn attach_and_snapshot(
    &self,
    command: AttachCommand,
  ) -> AttachmentStoreFuture<'_, PresenceAttachOutcome>;

  fn detach(&self, command: DetachCommand) -> AttachmentStoreFuture<'_, CommittedTransition>;

  fn disconnect(
    &self,
    command: DisconnectConnectionCommand,
  ) -> AttachmentStoreFuture<'_, Vec<CommittedTransition>>;
}
