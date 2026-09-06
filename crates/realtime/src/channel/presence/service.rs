use crate::channel::presence::command::PresenceBatchCommand;
use crate::{ChannelCommitDelivery, ChannelKey, PresenceError, PresenceMutationOutcome, PresenceMutationReceipt, PresenceSnapshot, PresenceStore};
use std::sync::Arc;

pub struct PresenceService {
  store: Arc<dyn PresenceStore>,
  delivery: Arc<dyn ChannelCommitDelivery>,
}

impl PresenceService {
  pub fn new(store: Arc<dyn PresenceStore>, delivery: Arc<dyn ChannelCommitDelivery>) -> Self {
    Self { store, delivery }
  }

  /// Применяет клиентскую Presence-команду и доставляет зафиксированный переход.
  ///
  /// Возвращает receipt целиком: `replayed` говорит вызывающему, что клиент
  /// повторил уже обработанную команду и получил прежний результат. Доставка
  /// при повторе выполняется снова намеренно — это способ пережить сбой
  /// первой доставки; за идемпотентность повтора отвечает `ChannelCommitDelivery`.
  pub async fn apply(
    &self,
    command: PresenceBatchCommand,
  ) -> Result<PresenceMutationReceipt, PresenceError> {
    let receipt = self.store.apply_presence(command).await?;

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
