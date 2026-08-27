use thiserror::Error;

/// Ошибка конфигурации [`crate::IncomingEventProcessor`], обнаруженная до
/// запуска обработки событий.
#[derive(Debug, Eq, Error, PartialEq)]
#[non_exhaustive]
pub enum IncomingEventProcessorConfigError {
  /// Область дедупликации не задана.
  #[error("incoming event processor scope must not be empty")]
  EmptyScope,

  /// Право на обработку немедленно истекало бы.
  #[error("incoming event processor lease_ttl must be greater than zero")]
  ZeroLeaseTtl,

  /// Отметка об успешно обработанном событии немедленно исчезала бы.
  #[error("incoming event processor completed_record_ttl must be greater than zero")]
  ZeroCompletedRecordTtl,
}
