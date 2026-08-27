use thiserror::Error;

use crate::{EventMessageError, HandlerError, ProcessingErrorClass};

/// Ошибка применения уже полученного [`crate::EventMessage`] к локальному
/// обязательному handler-у.
///
/// `DispatchError` относится только к входящей обработке. Ошибки публикации в
/// транспорт остаются в [`crate::EventBusError`]. Такое разделение позволяет
/// consumer-у принимать решение об `ACK`, повторе или окончательном отклонении
/// события, не смешивая его с исходящей публикацией.
#[derive(Debug, Error)]
pub enum DispatchError {
    /// Envelope невозможно проверить или преобразовать в зарегистрированный
    /// тип события.
    ///
    /// Ошибки содержимого envelope считаются постоянными: повторная доставка
    /// тех же байтов не изменит имя, версию или payload.
    #[error(transparent)]
    EventMessage(#[from] EventMessageError),

    /// Для имени события не найден обязательный handler.
    ///
    /// Это ошибка конфигурации запущенного приложения. Повторная доставка в тот
    /// же процесс не зарегистрирует handler, поэтому ошибка классифицируется как
    /// постоянная.
    #[error("handler for event {event_name} is not registered")]
    HandlerNotRegistered { event_name: String },

    /// Зарегистрированный handler был запущен, но завершился ошибкой.
    ///
    /// Класс берётся из [`HandlerError`], созданной самим handler-ом.
    #[error("handler for event {event_name} failed: {source}")]
    Handler {
        event_name: String,

        #[source]
        source: HandlerError,
    },
}

impl DispatchError {
    /// Возвращает класс ошибки, по которому consumer выбирает retry или
    /// окончательное отклонение события.
    pub const fn class(&self) -> ProcessingErrorClass {
        match self {
            Self::EventMessage(_) | Self::HandlerNotRegistered { .. } => {
                ProcessingErrorClass::Permanent
            }
            Self::Handler { source, .. } => source.class(),
        }
    }

    /// Возвращает `true`, если обработку события имеет смысл повторить.
    pub const fn is_retryable(&self) -> bool {
        matches!(self.class(), ProcessingErrorClass::Retryable)
    }

    /// Возвращает `true`, если повторная обработка тех же данных не поможет.
    pub const fn is_permanent(&self) -> bool {
        matches!(self.class(), ProcessingErrorClass::Permanent)
    }
}
