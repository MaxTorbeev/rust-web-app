use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IncomingEventOutcome {
  /// Handler выполнен, результат дедупликации сохранён.
  Applied,

  /// Событие уже было обработано, handler повторно не запускался.
  Duplicate,

  /// Событие обрабатывает другой исполнитель.
  InProgress { retry_after: Duration },
}
