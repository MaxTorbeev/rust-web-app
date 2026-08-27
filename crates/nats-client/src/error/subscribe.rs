use async_nats::jetstream::{
  consumer::StreamError as DriverMessageStreamError,
  context::GetStreamError as DriverGetStreamError, stream::ConsumerError as DriverConsumerError,
};
use thiserror::Error;

/// Error returned while opening or validating a JetStream consumer subscription.
///
/// Ошибка открытия подписки JetStream consumer-а или проверки его
/// конфигурации.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct SubscribeError(SubscribeErrorSource);

#[derive(Debug, Error)]
enum SubscribeErrorSource {
  #[error("failed to get JetStream stream: {0}")]
  Stream(#[source] DriverGetStreamError),

  #[error("failed to get or create JetStream consumer: {0}")]
  Consumer(#[source] DriverConsumerError),

  #[error("JetStream consumer `{durable_name}` has incompatible fields: {fields:?}")]
  IncompatibleConfiguration {
    durable_name: String,
    fields: Vec<&'static str>,
  },

  #[error("failed to open JetStream message stream: {0}")]
  Messages(#[source] DriverMessageStreamError),
}

impl SubscribeError {
  /// Returns `true` when an existing consumer differs from the requested
  /// configuration.
  ///
  /// Возвращает `true`, если конфигурация существующего consumer-а отличается
  /// от запрошенной.
  pub fn is_incompatible_configuration(&self) -> bool {
    matches!(
      self.0,
      SubscribeErrorSource::IncompatibleConfiguration { .. }
    )
  }

  pub(crate) fn stream(source: DriverGetStreamError) -> Self {
    Self(SubscribeErrorSource::Stream(source))
  }

  pub(crate) fn consumer(source: DriverConsumerError) -> Self {
    Self(SubscribeErrorSource::Consumer(source))
  }

  pub(crate) fn incompatible_configuration(
    durable_name: impl Into<String>,
    fields: Vec<&'static str>,
  ) -> Self {
    Self(SubscribeErrorSource::IncompatibleConfiguration {
      durable_name: durable_name.into(),
      fields,
    })
  }

  pub(crate) fn messages(source: DriverMessageStreamError) -> Self {
    Self(SubscribeErrorSource::Messages(source))
  }
}
