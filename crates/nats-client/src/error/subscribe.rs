use async_nats::jetstream::{
  consumer::StreamError as DriverMessageStreamError,
  context::GetStreamError as DriverGetStreamError,
  stream::ConsumerError as DriverConsumerError,
};
use thiserror::Error;

#[derive(Debug, Error)]
#[error(transparent)]
pub struct SubscribeError(SubscribeErrorSource);

#[derive(Debug, Error)]
enum SubscribeErrorSource {
  #[error("failed to get JetStream stream: {0}")]
  Stream(#[source] DriverGetStreamError),

  #[error("failed to get or create JetStream consumer: {0}")]
  Consumer(#[source] DriverConsumerError),

  #[error("failed to open JetStream message stream: {0}")]
  Messages(#[source] DriverMessageStreamError),
}

impl SubscribeError {
  pub(crate) fn stream(source: DriverGetStreamError) -> Self {
    Self(SubscribeErrorSource::Stream(source))
  }

  pub(crate) fn consumer(source: DriverConsumerError) -> Self {
    Self(SubscribeErrorSource::Consumer(source))
  }

  pub(crate) fn messages(source: DriverMessageStreamError) -> Self {
    Self(SubscribeErrorSource::Messages(source))
  }
}