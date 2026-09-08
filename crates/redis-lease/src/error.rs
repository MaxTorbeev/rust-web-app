use redis_client::{RedisClientError, ScriptValue};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RedisLeaseError {
  #[error(transparent)]
  Redis(#[from] RedisClientError),

  #[error("lease {field} must not be empty")]
  Empty { field: &'static str },

  #[error("lease TTL must be greater than zero")]
  ZeroTtl,

  #[error("lease TTL of {milliseconds} milliseconds exceeds the Redis limit")]
  TtlOverflow { milliseconds: u128 },

  #[error("unexpected Redis {operation} script value: {value:?}")]
  UnexpectedScriptValue {
    operation: &'static str,
    value: ScriptValue,
  },
}
