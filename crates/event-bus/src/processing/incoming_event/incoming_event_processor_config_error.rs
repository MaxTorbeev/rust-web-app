use std::time::Duration;

use thiserror::Error;

/// Ошибка [`crate::IncomingEventProcessorConfig`], обнаруженная до
/// запуска обработки событий.
#[derive(Debug, Eq, Error, PartialEq)]
#[non_exhaustive]
pub enum IncomingEventProcessorConfigError {
  /// Область дедупликации не задана.
  #[error("incoming event processor scope must not be empty")]
  EmptyScope,

  /// Handler немедленно достигал бы ограничения времени.
  #[error("incoming event processor processing_timeout must be greater than zero")]
  ZeroProcessingTimeout,

  /// Право на обработку немедленно истекало бы.
  #[error("incoming event processor lease_ttl must be greater than zero")]
  ZeroLeaseTtl,

  /// Отметка об успешно обработанном событии немедленно исчезала бы.
  #[error("incoming event processor completed_record_ttl must be greater than zero")]
  ZeroCompletedRecordTtl,

  /// Handler мог бы продолжить работу после истечения временного права на
  /// обработку события.
  #[error(
    "incoming event processor processing_timeout ({processing_timeout:?}) must be less than lease_ttl ({lease_ttl:?})"
  )]
  ProcessingTimeoutNotLessThanLeaseTtl {
    processing_timeout: Duration,
    lease_ttl: Duration,
  },
}
