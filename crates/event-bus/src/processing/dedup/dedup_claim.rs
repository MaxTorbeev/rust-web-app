use std::time::Duration;
use crate::DedupLease;

pub enum DedupClaim {
  /// Событие сейчас никто не обрабатывает
  /// Handler можно запускать
  Acquired(DedupLease),
  /// Событие уже было обработано ранее
  /// Handler запускать нельзя.
  Completed,
  /// Событие уже обрабатывает другой обработчик.
  /// Следует после `retry_after` повторить claim().
  InProgress {
    retry_after: Duration,
  },
}
