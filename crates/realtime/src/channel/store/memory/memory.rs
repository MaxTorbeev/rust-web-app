use support::timestamp::Timestamp;
use tokio::sync::Mutex;
use super::state::MemoryStoreState;
use crate::connection::DisconnectConnectionCommand;
use crate::{
  AttachCommand, AttachmentStore, AttachmentStoreFuture, ChannelAttachOutcome, ChannelKey,
  CommittedChannelTransition, DetachCommand, PresenceBatchCommand, PresenceMutationReceipt,
  PresenceLedgerPolicy, PresenceSnapshot, PresenceStore, PresenceStoreFuture,
};

/// Локальное хранилище состояния каналов.
pub struct MemoryChannelStore {
  state: Mutex<MemoryStoreState>,
}

impl MemoryChannelStore {
  pub fn new() -> Self {
    Self::with_ledger_policy(PresenceLedgerPolicy::default())
  }

  /// Создаёт хранилище с заданными ограничениями журналов операций.
  pub fn with_ledger_policy(policy: PresenceLedgerPolicy) -> Self {
    Self {
      state: Mutex::new(MemoryStoreState::new(policy)),
    }
  }

  /// Удаляет журналы операций соединений с истёкшим retention.
  ///
  /// Очистка уже выполняется при каждом disconnect; этот метод позволяет
  /// дополнительно вызывать её по таймеру. Возвращает число удалённых журналов.
  pub async fn sweep_presence_ledgers(&self, now: Timestamp) -> usize {
    self.state.lock().await.sweep_presence_ledgers(now)
  }
}

impl Default for MemoryChannelStore {
  fn default() -> Self {
    Self::new()
  }
}

impl PresenceStore for MemoryChannelStore {
  fn apply_presence(
    &self,
    command: PresenceBatchCommand,
  ) -> PresenceStoreFuture<'_, PresenceMutationReceipt> {
    Box::pin(async move { self.state.lock().await.apply_presence(command) })
  }

  fn snapshot(&self, channel: ChannelKey) -> PresenceStoreFuture<'_, PresenceSnapshot> {
    Box::pin(async move { Ok(self.state.lock().await.channel_snapshot(&channel)) })
  }
}

impl AttachmentStore for MemoryChannelStore {
  fn attach_and_snapshot(
    &self,
    command: AttachCommand,
  ) -> AttachmentStoreFuture<'_, ChannelAttachOutcome> {
    Box::pin(async move { self.state.lock().await.attach(command) })
  }

  fn detach(&self, command: DetachCommand) -> AttachmentStoreFuture<'_, CommittedChannelTransition> {
    Box::pin(async move { self.state.lock().await.detach(command) })
  }

  fn disconnect(
    &self,
    command: DisconnectConnectionCommand,
  ) -> AttachmentStoreFuture<'_, Vec<CommittedChannelTransition>> {
    Box::pin(async move { self.state.lock().await.disconnect(command) })
  }
}
