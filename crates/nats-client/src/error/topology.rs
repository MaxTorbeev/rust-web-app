use async_nats::connection::State as DriverConnectionState;
use async_nats::jetstream::context::{
  ConsumerInfoError as DriverConsumerInfoError, GetStreamError as DriverGetStreamError,
};
use thiserror::Error;

/// Error returned by a read-only JetStream topology verification.
///
/// Ошибка read-only проверки доступности и конфигурации JetStream stream-а и
/// durable consumer-а. Типы драйвера остаются скрыты внутри ошибки.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct TopologyError(TopologyErrorSource);

#[derive(Debug, Error)]
enum TopologyErrorSource {
  #[error("NATS connection is not connected: {state}")]
  CoreUnavailable { state: DriverConnectionState },

  #[error("failed to get JetStream stream `{stream_name}` info: {source}")]
  StreamInfo {
    stream_name: String,
    #[source]
    source: DriverGetStreamError,
  },

  #[error("JetStream stream `{stream_name}` has incompatible fields: {fields:?}")]
  StreamConfiguration {
    stream_name: String,
    fields: Vec<&'static str>,
  },

  #[error("failed to get JetStream consumer `{durable_name}` info: {source}")]
  ConsumerInfo {
    durable_name: String,
    #[source]
    source: DriverConsumerInfoError,
  },

  #[error("JetStream consumer `{durable_name}` has incompatible fields: {fields:?}")]
  ConsumerConfiguration {
    durable_name: String,
    fields: Vec<&'static str>,
  },

  #[error(
    "JetStream consumer `{durable_name}` belongs to stream `{actual}`, expected `{expected}`"
  )]
  ConsumerStreamMismatch {
    durable_name: String,
    actual: String,
    expected: String,
  },
}

impl TopologyError {
  pub(crate) fn core_unavailable(state: DriverConnectionState) -> Self {
    Self(TopologyErrorSource::CoreUnavailable { state })
  }

  pub(crate) fn stream_info(stream_name: impl Into<String>, source: DriverGetStreamError) -> Self {
    Self(TopologyErrorSource::StreamInfo {
      stream_name: stream_name.into(),
      source,
    })
  }

  pub(crate) fn stream_configuration(
    stream_name: impl Into<String>,
    fields: Vec<&'static str>,
  ) -> Self {
    Self(TopologyErrorSource::StreamConfiguration {
      stream_name: stream_name.into(),
      fields,
    })
  }

  pub(crate) fn consumer_info(
    durable_name: impl Into<String>,
    source: DriverConsumerInfoError,
  ) -> Self {
    Self(TopologyErrorSource::ConsumerInfo {
      durable_name: durable_name.into(),
      source,
    })
  }

  pub(crate) fn consumer_configuration(
    durable_name: impl Into<String>,
    fields: Vec<&'static str>,
  ) -> Self {
    Self(TopologyErrorSource::ConsumerConfiguration {
      durable_name: durable_name.into(),
      fields,
    })
  }

  pub(crate) fn consumer_stream_mismatch(
    durable_name: impl Into<String>,
    actual: impl Into<String>,
    expected: impl Into<String>,
  ) -> Self {
    Self(TopologyErrorSource::ConsumerStreamMismatch {
      durable_name: durable_name.into(),
      actual: actual.into(),
      expected: expected.into(),
    })
  }
}
