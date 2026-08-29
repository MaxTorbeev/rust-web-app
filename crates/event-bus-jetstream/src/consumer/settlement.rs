use std::time::Duration;

use nats_client::NatsMessage;

use super::error::JetStreamConsumerError;

/// Решение о дальнейшей судьбе текущей доставки JetStream.
///
/// Определяет, нужно ли подтвердить доставку, повторить её позднее
/// или окончательно прекратить повторные попытки.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SettlementAction {
  /// Подтвердить успешную обработку и прекратить повторную доставку.
  Ack,

  /// Запросить повторную доставку после указанной задержки.
  Nak { delay: Duration },

  /// Окончательно прекратить повторные доставки этому consumer-у.
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
