use std::time::Duration;

use crate::IncomingEventProcessorConfigError;

/// Настройки обработки входящих событий.
///
/// Конфигурация создаётся отдельно от [`crate::IncomingEventProcessor`], чтобы
/// все временные ограничения были проверены до запуска consumer-а.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncomingEventProcessorConfig {
  scope: String,
  processing_timeout: Duration,
  lease_ttl: Duration,
  completed_record_ttl: Duration,
}

impl IncomingEventProcessorConfig {
  /// Создаёт и проверяет настройки processor-а.
  ///
  /// `scope` вместе с `event_id` образует ключ дедупликации. Для `AllNodes`
  /// каждая нода должна использовать собственный стабильный `scope`, а для
  /// `WorkQueue` все consumers одной группы — общий `scope`.
  ///
  /// `processing_timeout` ограничивает время выполнения обязательного
  /// handler-а. `lease_ttl` задаёт срок временного права на обработку события
  /// и обязан быть строго больше `processing_timeout`.
  ///
  /// Эта проверка задаёт только минимально допустимое соотношение. В рабочей
  /// конфигурации между значениями нужен запас на доставку результата `claim`,
  /// выполнение `complete` и задержки хранилища. Строго гарантировать действие
  /// lease одним сравнением нельзя: его TTL начинает уменьшаться внутри
  /// хранилища ещё до того, как `claim` вернётся processor-у.
  ///
  /// `completed_record_ttl` определяет, как долго успешно обработанное событие
  /// распознаётся как дубликат.
  pub fn try_new(
    scope: impl Into<String>,
    processing_timeout: Duration,
    lease_ttl: Duration,
    completed_record_ttl: Duration,
  ) -> Result<Self, IncomingEventProcessorConfigError> {
    let scope = scope.into();

    if scope.is_empty() {
      return Err(IncomingEventProcessorConfigError::EmptyScope);
    }

    if processing_timeout.is_zero() {
      return Err(IncomingEventProcessorConfigError::ZeroProcessingTimeout);
    }

    if lease_ttl.is_zero() {
      return Err(IncomingEventProcessorConfigError::ZeroLeaseTtl);
    }

    if completed_record_ttl.is_zero() {
      return Err(IncomingEventProcessorConfigError::ZeroCompletedRecordTtl);
    }

    if processing_timeout >= lease_ttl {
      return Err(
        IncomingEventProcessorConfigError::ProcessingTimeoutNotLessThanLeaseTtl {
          processing_timeout,
          lease_ttl,
        },
      );
    }

    Ok(Self {
      scope,
      processing_timeout,
      lease_ttl,
      completed_record_ttl,
    })
  }

  /// Стабильная область, внутри которой `event_id` должен обрабатываться один
  /// раз.
  pub fn scope(&self) -> &str {
    &self.scope
  }

  /// Кооперативное ограничение времени выполнения обязательного handler-а.
  pub const fn processing_timeout(&self) -> Duration {
    self.processing_timeout
  }

  /// Срок временного исключительного права на обработку события.
  pub const fn lease_ttl(&self) -> Duration {
    self.lease_ttl
  }

  /// Срок хранения отметки об успешно обработанном событии.
  pub const fn completed_record_ttl(&self) -> Duration {
    self.completed_record_ttl
  }
}
