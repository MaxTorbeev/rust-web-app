use thiserror::Error;

/// Ошибка конфигурации входящего JetStream consumer-а.
#[derive(Debug, Eq, Error, PartialEq)]
#[non_exhaustive]
pub enum JetStreamIncomingConsumerConfigError {
  /// Нулевая задержка создала бы горячий цикл повторных доставок.
  #[error("JetStream incoming consumer retry_delay must be greater than zero")]
  ZeroRetryDelay,
}
