use async_nats::jetstream::consumer::pull::{
  Stream as DriverMessageStream,
};
use futures_util::StreamExt;
use crate::{NatsMessage, ReceiveError};

pub struct NatsSubscription {
  messages: DriverMessageStream,
}

impl NatsSubscription {
  pub(crate) fn new(messages: DriverMessageStream) -> Self {
    Self { messages }
  }

  pub(crate) async fn next(&mut self) -> Option<Result<NatsMessage, ReceiveError>> {
    self.messages.next().await.map(|result| {
      result
        .map(NatsMessage::from_driver)
        .map_err(ReceiveError::from_driver)
    })
  }
}