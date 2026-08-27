use crate::DedupLease;
use std::time::Duration;

#[derive(Debug, Eq, PartialEq)]
pub enum DedupClaim {
  /// Событие сейчас никто не обрабатывает
  /// Handler можно запускать
  Acquired(DedupLease),
  /// Событие уже было обработано ранее
  /// Handler запускать нельзя.
  Completed,
  /// Событие уже обрабатывает другой обработчик.
  /// Следует после `retry_after` повторить claim().
  InProgress { retry_after: Duration },
}
