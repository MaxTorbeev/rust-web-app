use thiserror::Error;

/// Ошибка создания, восстановления или типизированного чтения
/// [`crate::EventMessage`].
///
/// Тип отделён от [`crate::EventBusError`], потому что envelope используется не
/// только при публикации. Исходящий publisher сериализует его в байты, а
/// входящий consumer восстанавливает из байтов и передаёт dispatcher-у.
#[derive(Debug, Error)]
pub enum EventMessageError {
    /// Доменное событие или готовый envelope невозможно сериализовать.
    #[error("failed to encode event message: {0}")]
    Encode(#[source] serde_json::Error),

    /// Envelope или его payload невозможно десериализовать.
    #[error("failed to decode event message: {0}")]
    Decode(#[source] serde_json::Error),

    /// Dispatcher попытался прочитать envelope как событие другого типа.
    #[error("event type mismatch: expected {expected}, got {actual}")]
    EventTypeMismatch { expected: String, actual: String },

    /// Версия payload не совпадает с версией зарегистрированного типа события.
    #[error("event version mismatch for {event_name}: expected {expected}, got {actual}")]
    EventVersionMismatch {
        event_name: String,
        expected: u16,
        actual: u16,
    },
}
