use async_nats::jetstream::consumer::pull::Stream as DriverMessageStream;
use futures_util::StreamExt;

use crate::{NatsMessage, ReceiveError};

/// Pull-consumer delivery stream.
///
/// Поток входящих доставок durable pull consumer-а JetStream.
pub struct NatsSubscription {
  messages: DriverMessageStream,
}

impl NatsSubscription {
  pub(crate) fn new(messages: DriverMessageStream) -> Self {
    Self { messages }
  }

  /// Receives the next delivery, or `None` when the underlying stream closes.
  ///
  /// Получает следующее сообщение. Возвращает `None`, когда underlying stream
  /// закрыт и новых доставок больше не будет.
  pub async fn next(&mut self) -> Option<Result<NatsMessage, ReceiveError>> {
    self.messages.next().await.map(|result| {
      result
        .map(NatsMessage::from_driver)
        .map_err(ReceiveError::from_driver)
    })
  }
}
