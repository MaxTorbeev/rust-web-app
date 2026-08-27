use thiserror::Error;

use crate::ProcessingErrorClass;

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Ошибка обращения к хранилищу дедупликации.
#[derive(Debug, Error)]
pub enum DedupStoreError {
  /// Хранилище не смогло выполнить операцию.
  ///
  /// Сюда оборачиваются ошибки Redis: отсутствие соединения, тайм-аут,
  /// ошибка выполнения Lua-скрипта или некорректный ответ сервера.
  #[error("deduplication store operation failed: {source}")]
  Backend {
    #[source]
    source: BoxError,
  },

  /// Исполнитель больше не владеет lease.
  ///
  /// Такое возможно, если lease истёк либо событие уже захватил другой
  /// исполнитель с новым `token`.
  #[error("deduplication lease is no longer owned")]
  LeaseLost,
}

impl DedupStoreError {
  
  /// Оборачивает ошибку конкретного хранилища.
  pub fn backend(error: impl std::error::Error + Send + Sync + 'static) -> Self {
    Self::Backend {
      source: Box::new(error),
    }
  }

  /// Ошибки хранилища не позволяют безопасно подтвердить обработку,
  /// поэтому доставку следует повторить.
  pub const fn class(&self) -> ProcessingErrorClass {
    ProcessingErrorClass::Retryable
  }
}