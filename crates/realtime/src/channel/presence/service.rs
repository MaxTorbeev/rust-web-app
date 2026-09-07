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
  /// повторил уже обработанную команду и получил прежний результат.
  ///
  /// Воспроизведённый результат не доставляется повторно. Replay означает, что
  /// событие уже было зафиксировано и передано в delivery при первой обработке;
  /// второй `after_commit` того же события дал бы подписчикам дубль deltas
  /// (потерян `ACK` → клиент повторил `PRESENCE` → все получили Enter дважды).
  /// Повторять доставку ради восстановления после сбоя первой попытки нет
  /// смысла: в memory-режиме `broadcast` либо доходит до всех, либо падает на
  /// сериализации кадра детерминированно — повтор даст ту же ошибку; в
  /// Redis-режиме доставка идёт через outbox, и `after_commit` — no-op.
  ///
  /// Следствие: если первая доставка упала, клиент получил `NACK`, а операция
  /// при этом уже зафиксирована и записана в журнал, повтор вернёт `ACK` без
  /// новой попытки доставки. Это симптом бага сериализации, не штатный путь.
  ///
  /// Идемпотентность самого `ChannelCommitDelivery` по `event_id` остаётся его
  /// контрактом (см. трейт) — этот метод на неё не полагается, но и не
  /// подменяет.
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
