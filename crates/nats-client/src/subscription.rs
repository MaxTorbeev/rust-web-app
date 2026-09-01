use std::future::poll_fn;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_nats::jetstream::consumer::pull::Stream as DriverMessageStream;
use futures_util::Stream;

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
    poll_fn(|context| self.poll_next(context)).await
  }

  /// Polls the underlying delivery stream once.
  ///
  /// This primitive lets a runtime observe that the pull consumer has actually
  /// entered its receive loop without waiting for the first message.
  pub fn poll_next(
    &mut self,
    context: &mut Context<'_>,
  ) -> Poll<Option<Result<NatsMessage, ReceiveError>>> {
    Pin::new(&mut self.messages)
      .poll_next(context)
      .map(|delivery| {
        delivery.map(|result| {
          result
            .map(NatsMessage::from_driver)
            .map_err(ReceiveError::from_driver)
        })
      })
  }
}
