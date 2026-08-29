use crate::error::RedisDedupStoreError;
use event_bus::DedupKey;
use redis_client::ScriptValue;
use std::time::Duration;
use support::app::APP_NAMESPACE_SEPARATOR;
use uuid::Uuid;

const LEASE_VALUE_PREFIX: &str = "lease:";
const REDIS_MILLISECOND_NANOS: u128 = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ClaimScriptOutcome {
  Acquired,
  Completed,
  InProgress { retry_after: Duration },
}

pub(crate) fn redis_key(prefix: &str, key: &DedupKey) -> String {
  let event_id = key.event_id().to_string();

  [prefix, key.scope(), event_id.as_str()].join(APP_NAMESPACE_SEPARATOR)
}

pub(crate) fn lease_value(token: Uuid) -> String {
  format!("{LEASE_VALUE_PREFIX}{token}")
}

pub(crate) fn redis_ttl_milliseconds(ttl: Duration) -> Result<i64, RedisDedupStoreError> {
  if ttl.is_zero() {
    return Err(RedisDedupStoreError::ZeroTtl);
  }

  let milliseconds = ttl.as_nanos().div_ceil(REDIS_MILLISECOND_NANOS);

  i64::try_from(milliseconds).map_err(|_| RedisDedupStoreError::TtlOverflow { milliseconds })
}

/// Декодер ответа claim.lua
pub(crate) fn decode_claim_value(
  value: ScriptValue,
) -> Result<ClaimScriptOutcome, RedisDedupStoreError> {
  let outcome = match &value {
    ScriptValue::Array(values) => match values.as_slice() {
      [ScriptValue::Integer(1)] => Some(ClaimScriptOutcome::Acquired),
      [ScriptValue::Integer(2)] => Some(ClaimScriptOutcome::Completed),
      [ScriptValue::Integer(3), ScriptValue::Integer(milliseconds)] => u64::try_from(*milliseconds)
        .ok()
        .map(|milliseconds| ClaimScriptOutcome::InProgress {
          retry_after: Duration::from_millis(milliseconds),
        }),
      _ => None,
    },
    _ => None,
  };

  outcome.ok_or(RedisDedupStoreError::UnexpectedClaimValue { value })
}

pub(crate) fn decode_transition_value(
  operation: &'static str,
  value: ScriptValue,
) -> Result<bool, RedisDedupStoreError> {
  match value {
    ScriptValue::Integer(1) => Ok(true),
    ScriptValue::Integer(0) => Ok(false),
    value => Err(RedisDedupStoreError::UnexpectedTransitionValue { operation, value }),
  }
}

#[cfg(test)]
mod tests;
