use std::time::Duration;

use async_nats::jetstream::consumer::{
  AckPolicy, Config as DriverBaseConsumerConfig, DeliverPolicy,
  pull::Config as DriverConsumerConfig,
};

use crate::error::ConsumerConfigError;
use crate::validation::{is_valid_entity_name, is_valid_subject_filter};

/// Configuration of one durable JetStream pull consumer.
///
/// Конфигурация одного durable pull consumer-а JetStream. Определяет источник
/// сообщений, стабильное имя consumer-а, фильтр, таймаут подтверждения и границы
/// повторной доставки.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsumerConfig {
  stream_name: String,
  durable_name: String,
  filter_subject: String,
  ack_wait: Duration,
  max_deliver: i64,
  max_ack_pending: i64,
}

impl ConsumerConfig {
  /// Builds a validated explicit-ack durable consumer configuration.
  ///
  /// Создаёт и проверяет конфигурацию durable consumer-а с явными ACK,
  /// ограниченным числом повторных доставок и лимитом сообщений без ACK.
  pub fn try_new(
    stream_name: impl Into<String>,
    durable_name: impl Into<String>,
    filter_subject: impl Into<String>,
    ack_wait: Duration,
    max_deliver: i64,
  ) -> Result<Self, ConsumerConfigError> {
    let stream_name = stream_name.into();
    let durable_name = durable_name.into();
    let filter_subject = filter_subject.into();

    if !is_valid_entity_name(&stream_name) {
      return Err(ConsumerConfigError::InvalidStreamName { value: stream_name });
    }

    if !is_valid_entity_name(&durable_name) {
      return Err(ConsumerConfigError::InvalidDurableName {
        value: durable_name,
      });
    }

    if !is_valid_subject_filter(&filter_subject) {
      return Err(ConsumerConfigError::InvalidFilterSubject {
        value: filter_subject,
      });
    }

    if ack_wait.is_zero() {
      return Err(ConsumerConfigError::ZeroAckWait);
    }

    if max_deliver <= 0 {
      return Err(ConsumerConfigError::InvalidMaxDeliver { max_deliver });
    }

    // Входящий consumer пока обрабатывает сообщения последовательно. Разрешаем
    // серверу выдать только одно неподтверждённое сообщение, чтобы `ack_wait`
    // следующих сообщений не истекал, пока они ожидают своей очереди.
    let max_ack_pending = 1;

    Ok(Self {
      stream_name,
      durable_name,
      filter_subject,
      ack_wait,
      max_deliver,
      max_ack_pending,
    })
  }

  pub(crate) fn stream_name(&self) -> &str {
    &self.stream_name
  }

  pub(crate) fn durable_name(&self) -> &str {
    &self.durable_name
  }

  pub(crate) fn to_driver_config(&self) -> DriverConsumerConfig {
    DriverConsumerConfig {
      durable_name: Some(self.durable_name.clone()),
      deliver_policy: DeliverPolicy::New,
      ack_policy: AckPolicy::Explicit,
      ack_wait: self.ack_wait,
      max_deliver: self.max_deliver,
      filter_subject: self.filter_subject.clone(),
      max_ack_pending: self.max_ack_pending,
      ..Default::default()
    }
  }

  pub(crate) fn incompatible_fields(&self, actual: &DriverBaseConsumerConfig) -> Vec<&'static str> {
    let mut fields = Vec::new();

    if actual.durable_name.as_deref() != Some(self.durable_name.as_str()) {
      fields.push("durable_name");
    }
    if actual.deliver_policy != DeliverPolicy::New {
      fields.push("deliver_policy");
    }
    if actual.ack_policy != AckPolicy::Explicit {
      fields.push("ack_policy");
    }
    if actual.ack_wait != self.ack_wait {
      fields.push("ack_wait");
    }
    if actual.max_deliver != self.max_deliver {
      fields.push("max_deliver");
    }
    if actual.filter_subject != self.filter_subject {
      fields.push("filter_subject");
    }
    if actual.max_ack_pending != self.max_ack_pending {
      fields.push("max_ack_pending");
    }

    fields
  }
}
