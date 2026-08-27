use thiserror::Error;

/// Ошибка регистрации обязательного обработчика события.
///
/// Регистрация выполняется один раз при запуске приложения, до того как
/// [`crate::EventDispatcher`] будет помещён в `Arc` и передан рабочим задачам.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum HandlerRegistrationError {
  /// Для стабильного wire-name события уже зарегистрирован обработчик.
  ///
  /// Dispatcher намеренно разрешает только один обязательный handler на одно
  /// имя события: именно его результат определяет успех обработки.
  #[error("handler for event {event_name} is already registered")]
  AlreadyRegistered { event_name: String },
}
