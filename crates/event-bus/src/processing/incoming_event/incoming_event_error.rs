use std::time::Duration;

use crate::{DedupStoreError, DispatchError, ProcessingErrorClass};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IncomingEventError {
  #[error("failed to claim incoming event: {source}")]
  Claim {
    #[source]
    source: DedupStoreError,
  },

  #[error("failed to dispatch incoming event: {source}")]
  Dispatch {
    #[source]
    source: DispatchError,

    // Ошибка release сохраняется для диагностики, но не заменяет
    // первоначальный класс DispatchError.
    release_error: Option<DedupStoreError>,
  },

  /// Обязательный handler не завершился за отведённое время.
  ///
  /// После срабатывания кооперативного таймера processor удаляет future
  /// handler-а и пытается освободить lease. Уже выполненные handler-ом побочные
  /// эффекты при этом не откатываются.
  #[error("incoming event processing timed out after {timeout:?}")]
  ProcessingTimeout {
    timeout: Duration,

    // Ошибка release сохраняется для диагностики, но таймаут остаётся
    // основной retryable-ошибкой.
    release_error: Option<DedupStoreError>,
  },

  #[error("failed to complete incoming event: {source}")]
  Complete {
    #[source]
    source: DedupStoreError,
  },
}

impl IncomingEventError {
  pub const fn class(&self) -> ProcessingErrorClass {
    match self {
      Self::Claim { source } | Self::Complete { source } => source.class(),
      Self::Dispatch { source, .. } => source.class(),
      Self::ProcessingTimeout { .. } => ProcessingErrorClass::Retryable,
    }
  }

  pub fn release_error(&self) -> Option<&DedupStoreError> {
    match self {
      Self::Dispatch { release_error, .. } | Self::ProcessingTimeout { release_error, .. } => {
        release_error.as_ref()
      }
      _ => None,
    }
  }
}
