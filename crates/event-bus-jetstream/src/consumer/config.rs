use std::time::Duration;
use crate::consumer::JetStreamIncomingConsumerConfigError;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JetStreamIncomingConsumerConfig {
  retry_delay: Duration,
}

impl JetStreamIncomingConsumerConfig {
  /// Создаёт конфигурацию и проверяет задержку повторной доставки.
  pub fn try_new(retry_delay: Duration) -> Result<Self, JetStreamIncomingConsumerConfigError> {
    if retry_delay.is_zero() {
      return Err(JetStreamIncomingConsumerConfigError::ZeroRetryDelay);
    }

    Ok(Self { retry_delay })
  }

  /// Задержка перед повторной доставкой после временной ошибки.
  pub const fn retry_delay(&self) -> Duration {
    self.retry_delay
  }
}
