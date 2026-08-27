use thiserror::Error;

use crate::{DispatchError, EventMessageError};

/// Ошибка публикации события через [`crate::EventBus`].
///
/// `EventBus` является исходящим API приложения, поэтому этот тип объединяет
/// только ошибки подготовки envelope, выбранного publisher-а и локального
/// dispatch-а. Детальный класс ошибки локальной обработки сохраняется внутри
/// [`DispatchError`].
#[derive(Debug, Error)]
pub enum EventBusError {
    /// EventMessage не удалось создать или сериализовать до публикации.
    #[error(transparent)]
    EventMessage(#[from] EventMessageError),

    /// Выбранный publisher не смог подтвердить публикацию.
    #[error("failed to publish event: {0}")]
    Publisher(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// Локальный publisher передал envelope dispatcher-у, и локальная обработка
    /// завершилась ошибкой.
    #[error(transparent)]
    Dispatch(#[from] DispatchError),
}

impl EventBusError {
    /// Оборачивает ошибку конкретного транспорта или publisher adapter-а.
    pub fn publisher(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Publisher(Box::new(error))
    }
}
