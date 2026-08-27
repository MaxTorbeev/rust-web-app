use thiserror::Error;
use crate::{DedupStoreError, DispatchError, ProcessingErrorClass};

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
    }
  }

  pub fn release_error(&self) -> Option<&DedupStoreError> {
    match self {
      Self::Dispatch { release_error, .. } => release_error.as_ref(),
      _ => None,
    }
  }
}