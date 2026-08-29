use redis_client::ScriptValue;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum RedisDedupStoreError {
  #[error("deduplication TTL must be greater than zero")]
  ZeroTtl,

  #[error("deduplication TTL of {milliseconds} milliseconds exceeds the Redis limit")]
  TtlOverflow { milliseconds: u128 },

  #[error("unexpected Redis claim script value: {value:?}")]
  UnexpectedClaimValue { value: ScriptValue },

  #[error("unexpected Redis {operation} script value: {value:?}")]
  UnexpectedTransitionValue {
    operation: &'static str,
    value: ScriptValue,
  },
}
