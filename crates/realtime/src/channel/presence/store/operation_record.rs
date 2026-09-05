use crate::PresenceMutationOutcome;

/// Сохранённый результат обработанной Presence-команды.
///
/// Запись неизменяема: результат первой обработки команды возвращается при
/// каждом её повторе, включая тот же `event_id` для зафиксированных операций.
#[derive(Clone, Debug)]
pub(crate) struct PresenceOperationRecord {
  /// Хеш содержимого первоначальной команды.
  request_fingerprint: String,

  /// Результат, который необходимо вернуть при повторе команды.
  outcome: PresenceMutationOutcome,
}

impl PresenceOperationRecord {
  pub(crate) fn new(request_fingerprint: String, outcome: PresenceMutationOutcome) -> Self {
    Self {
      request_fingerprint,
      outcome,
    }
  }

  /// Проверяет, что повтор содержит тот же normalized payload, что и
  /// первоначальная команда. Иначе повтор является protocol conflict.
  pub(crate) fn matches(&self, request_fingerprint: &str) -> bool {
    self.request_fingerprint == request_fingerprint
  }

  pub(crate) fn outcome(&self) -> &PresenceMutationOutcome {
    &self.outcome
  }
}
