use thiserror::Error;

use crate::ProcessingErrorClass;

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Ошибка, которую обязательный handler возвращает после получения
/// типизированного события.
///
/// Handler обязан явно выбрать класс ошибки. Благодаря этому входящий consumer
/// не анализирует текст ошибки и не пытается угадать, поможет ли повтор.
///
/// # Пример
///
/// Временная недоступность внешнего хранилища обычно является
/// [`Retryable`](Self::Retryable), а некорректная ссылка на несуществующее
/// приложение — [`Permanent`](Self::Permanent).
#[derive(Debug, Error)]
pub enum HandlerError {
  /// Обработка не завершилась из-за временной причины.
  #[error("retryable handler failure: {source}")]
  Retryable {
    #[source]
    source: BoxError,
  },

  /// Обработка не может завершиться успешно при повторе тех же данных.
  #[error("permanent handler failure: {source}")]
  Permanent {
    #[source]
    source: BoxError,
  },
}

impl HandlerError {
  /// Создаёт временную ошибку, после которой событие можно обработать повторно.
  pub fn retryable(error: impl std::error::Error + Send + Sync + 'static) -> Self {
    Self::Retryable {
      source: Box::new(error),
    }
  }

  /// Создаёт постоянную ошибку, для которой повтор с теми же данными бесполезен.
  pub fn permanent(error: impl std::error::Error + Send + Sync + 'static) -> Self {
    Self::Permanent {
      source: Box::new(error),
    }
  }

  /// Возвращает решение о возможности повторной обработки.
  pub const fn class(&self) -> ProcessingErrorClass {
    match self {
      Self::Retryable { .. } => ProcessingErrorClass::Retryable,
      Self::Permanent { .. } => ProcessingErrorClass::Permanent,
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
