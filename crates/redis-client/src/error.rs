use std::error::Error;
use std::fmt::{Display, Formatter};

type BoxError = Box<dyn Error + Send + Sync + 'static>;

/// Категория ошибки, которую вернул [`RedisClient`](crate::RedisClient).
///
/// Категория позволяет вызывающему коду различать этап сбоя, не зная о
/// конкретном Redis-драйвере и его типах ошибок.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RedisClientErrorKind {
  /// Не удалось создать клиент или установить соединение с Redis.
  Connection,
  /// Redis-команда, включая Lua-скрипт, завершилась ошибкой.
  Command,
  /// Redis вернул ответ, который не поддерживается публичным контрактом.
  Response,
}

impl Display for RedisClientErrorKind {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Connection => formatter.write_str("failed to connect to Redis"),
      Self::Command => formatter.write_str("Redis command failed"),
      Self::Response => formatter.write_str("invalid Redis response"),
    }
  }
}

/// Ошибка публичного Redis-клиента.
///
/// Исходная ошибка сохраняется в цепочке [`Error::source`], но её конкретный
/// тип является внутренней деталью `redis-client`. Другим crate не требуется
/// зависеть от используемого Redis-драйвера.
#[derive(Debug, thiserror::Error)]
#[error("{kind}: {source}")]
pub struct RedisClientError {
  kind: RedisClientErrorKind,
  #[source]
  source: BoxError,
}

impl RedisClientError {
  /// Возвращает категорию ошибки без раскрытия типа внутреннего драйвера.
  pub const fn kind(&self) -> RedisClientErrorKind {
    self.kind
  }

  pub(crate) fn connection(source: impl Error + Send + Sync + 'static) -> Self {
    Self::new(RedisClientErrorKind::Connection, source)
  }

  pub(crate) fn command(source: impl Error + Send + Sync + 'static) -> Self {
    Self::new(RedisClientErrorKind::Command, source)
  }

  pub(crate) fn unsupported_script_value(value_kind: &'static str) -> Self {
    Self::new(
      RedisClientErrorKind::Response,
      UnsupportedScriptValueError { value_kind },
    )
  }

  fn new(kind: RedisClientErrorKind, source: impl Error + Send + Sync + 'static) -> Self {
    Self {
      kind,
      source: Box::new(source),
    }
  }
}

/// Результат операции [`RedisClient`](crate::RedisClient).
pub type RedisClientResult<T> = Result<T, RedisClientError>;

#[derive(Debug, thiserror::Error)]
#[error("unsupported Lua response value: {value_kind}")]
struct UnsupportedScriptValueError {
  value_kind: &'static str,
}
