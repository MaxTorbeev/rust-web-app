use crate::PresenceMutationOutcome;

/// Сохранённый результат обработанной Presence-команды.
#[derive(Clone, Debug)]
pub(crate) struct PresenceOperationRecord {
  /// Хеш содержимого первоначальной команды.
  pub(crate) request_fingerprint: String,

  /// Результат, который необходимо вернуть при повторе команды.
  pub(crate) outcome: PresenceMutationOutcome,
}
