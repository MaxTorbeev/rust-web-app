use crate::channel::presence::command::PresenceBatchCommand;
use crate::{
  ChannelCommitDelivery, ChannelKey, PresenceError, PresenceMutationOutcome,
  PresenceMutationReceipt, PresenceSnapshot, PresenceStore,
};
use std::sync::Arc;

pub struct PresenceService {
  store: Arc<dyn PresenceStore>,
  delivery: Arc<dyn ChannelCommitDelivery>,
}

impl PresenceService {
  pub(crate) async fn project(
    &self,
    router: &crate::ChannelRouter,
    change: &crate::PresenceChannelChanged,
  ) -> Result<(), crate::ChannelCommitDeliveryError> {
    router.project_presence(change, self.store.as_ref()).await
  }
  pub fn new(store: Arc<dyn PresenceStore>, delivery: Arc<dyn ChannelCommitDelivery>) -> Self {
    Self { store, delivery }
  }

  /// Применяет клиентскую Presence-команду и доставляет зафиксированный переход.
  ///
  /// Возвращает receipt целиком: `replayed` говорит вызывающему, что клиент
  /// повторил уже обработанную команду и получил прежний результат.
  ///
  /// Replay возвращает прежний receipt без повторного вызова delivery.
  /// В Redis-режиме доставка независимо повторяется из durable outbox;
  /// `after_commit` ничего не публикует. В memory-режиме ошибки первой
  /// доставки не восстанавливаются повтором клиентской команды.
  pub async fn apply(
    &self,
    command: PresenceBatchCommand,
  ) -> Result<PresenceMutationReceipt, PresenceError> {
    let receipt = self.store.apply_presence(command).await?;

    if receipt.replayed {
      return Ok(receipt);
    }

    if let PresenceMutationOutcome::Committed(transition) = &receipt.outcome {
      self.delivery.after_commit(transition).await?;
    }

    Ok(receipt)
  }

  /// Возвращает текущий снимок Presence и Occupancy канала.
  pub async fn snapshot(&self, channel: ChannelKey) -> Result<PresenceSnapshot, PresenceError> {
    Ok(self.store.snapshot(channel).await?)
  }
}
