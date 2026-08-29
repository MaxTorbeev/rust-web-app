use crate::{JetStreamConsumerError, NatsMessage};
use std::time::Duration;

/// Решение о дальнейшей судьбе текущей доставки JetStream.
///
/// Определяет, нужно ли подтвердить доставку, повторить её позднее
/// или окончательно прекратить повторные попытки.
pub enum SettlementAction {
  /// Обработка завершена, удалить доставку из очереди
  Ack,
  /// Сейчас обработать нельзя, доставить повторно после delay
  Nak { delay: Duration },
  /// рекратить повторные доставки этому consumer-у
  Terminate,
}

impl SettlementAction {
  pub(crate) async fn apply(self, delivery: &NatsMessage) -> Result<(), JetStreamConsumerError> {
    match self {
      Self::Ack => delivery.ack().await.map_err(JetStreamConsumerError::Ack),

      Self::Nak { delay } => delivery
        .nak(Some(delay))
        .await
        .map_err(JetStreamConsumerError::Nak),

      Self::Terminate => delivery.term().await.map_err(JetStreamConsumerError::Term),
    }
  }
}
