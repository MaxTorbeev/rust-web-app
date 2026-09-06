/// Готовность приложения принимать новый трафик.
///
/// Относится ко всему приложению и не является состоянием EventBus.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TrafficState {
  /// Обычная работа: новые HTTP и WebSocket-соединения принимаются.
  Accepting,

  /// Graceful shutdown начат: readiness возвращает `503`, liveness — `200`.
  ///
  /// Переход в это состояние появится вместе с application-level draining;
  /// сейчас приложение всегда находится в `Accepting`.
  #[allow(dead_code)]
  Draining,
}

impl TrafficState {
  pub(crate) const fn is_accepting(self) -> bool {
    matches!(self, Self::Accepting)
  }
}
