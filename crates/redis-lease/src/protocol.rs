//! Раскладка lease в Redis: форма значения, ключ fence, единицы TTL и ответы
//! скриптов.
//!
//! Функции подготовки аргументов и разбора ответов используются также
//! адаптерами, которые встраивают операции lease в составные Lua-скрипты.

use std::time::Duration;

use redis_client::ScriptValue;

use crate::error::RedisLeaseError;
use crate::identity::{LeaseKey, LeaseOwner, LeaseToken};
use crate::outcome::{AcquireOutcome, Fence, ReleaseOutcome, RenewOutcome};

const LEASE_VALUE_PREFIX: &str = "lease:";
const FENCE_SEPARATOR: &str = ":";
const FENCE_KEY_SUFFIX: &str = ":fence";
const REDIS_MILLISECOND_NANOS: u128 = 1_000_000;

/// Значение lease-ключа для периода владения — то, что хранится в Redis и что
/// чужие скрипты передают в `holds_lease`.
///
/// Формат `lease:<owner>:<fence>` является публичным контрактом: его читают
/// Lua-скрипты других крейтов, изменение формата — изменение протокола. Fence —
/// последний сегмент и состоит только из цифр; владелец может содержать `:`,
/// поэтому значение разбирается с конца.
///
/// Значение идентифицирует период, а не владельца: token прошлого периода того
/// же владельца с ним не совпадает (см. [`LeaseToken`]).
pub fn lease_value(token: &LeaseToken) -> String {
  format!(
    "{}{FENCE_SEPARATOR}{}",
    owner_value(token.owner()),
    token.fence().get()
  )
}

/// Значение владельца без fence: `lease:<owner>`.
///
/// Используется при захвате lease. Для продления, освобождения
/// и проверки владения нужен полный token.
pub fn owner_value(owner: &LeaseOwner) -> String {
  format!("{LEASE_VALUE_PREFIX}{}", owner.as_str())
}

/// Ключ монотонного счётчика fence, живущий рядом с lease-ключом.
///
/// Хранится отдельно и без TTL: fence должен переживать release и истечение
/// lease, иначе новый владелец мог бы получить fence не больше прежнего.
pub fn fence_key(key: &LeaseKey) -> String {
  format!("{}{FENCE_KEY_SUFFIX}", key.as_str())
}

/// Переводит положительный TTL в миллисекунды с округлением вверх.
/// Возвращает ошибку для нулевого TTL или значения за пределами `i64`.
pub fn redis_ttl_milliseconds(ttl: Duration) -> Result<i64, RedisLeaseError> {
  if ttl.is_zero() {
    return Err(RedisLeaseError::ZeroTtl);
  }

  let milliseconds = ttl.as_nanos().div_ceil(REDIS_MILLISECOND_NANOS);

  i64::try_from(milliseconds).map_err(|_| RedisLeaseError::TtlOverflow { milliseconds })
}

/// Ответ `acquire.lua`: `{1, fence}` — lease у вызывающего, `{2, remaining_ms}`
/// — у другого владельца. Fence передаётся точной десятичной строкой
/// в диапазоне `1..=i64::MAX`, который поддерживает Redis INCR.
///
/// Token собирается здесь из ключа и владельца запроса и fence из ответа: в
/// Redis лежит то же самое в виде `lease:<owner>:<fence>`.
pub fn decode_acquire(
  value: ScriptValue,
  key: &LeaseKey,
  owner: &LeaseOwner,
) -> Result<AcquireOutcome, RedisLeaseError> {
  let unexpected = |value| RedisLeaseError::UnexpectedScriptValue {
    operation: "acquire",
    value,
  };

  let ScriptValue::Array(values) = &value else {
    return Err(unexpected(value));
  };

  match values.as_slice() {
    [ScriptValue::Integer(1), ScriptValue::Bytes(bytes)] => {
      let text = std::str::from_utf8(bytes).map_err(|_| unexpected(value.clone()))?;
      let fence = text.parse::<i64>().map_err(|_| unexpected(value.clone()))?;

      if fence <= 0 || fence.to_string() != text {
        return Err(unexpected(value.clone()));
      }

      Ok(AcquireOutcome::Acquired {
        token: LeaseToken::new(key.clone(), owner.clone(), Fence::new(fence as u64)),
      })
    }
    [ScriptValue::Integer(2), ScriptValue::Integer(remaining_ms)] => {
      let remaining_ms = u64::try_from(*remaining_ms).map_err(|_| unexpected(value.clone()))?;

      Ok(AcquireOutcome::Held {
        remaining: Duration::from_millis(remaining_ms),
      })
    }
    _ => Err(unexpected(value)),
  }
}

/// Разбирает ответ продления lease: `1` — продлён, `0` — владение потеряно.
pub fn decode_renew(value: ScriptValue) -> Result<RenewOutcome, RedisLeaseError> {
  match value {
    ScriptValue::Integer(1) => Ok(RenewOutcome::Renewed),
    ScriptValue::Integer(0) => Ok(RenewOutcome::Lost),
    value => Err(RedisLeaseError::UnexpectedScriptValue {
      operation: "renew",
      value,
    }),
  }
}

pub(crate) fn decode_release(value: ScriptValue) -> Result<ReleaseOutcome, RedisLeaseError> {
  match value {
    ScriptValue::Integer(1) => Ok(ReleaseOutcome::Released),
    ScriptValue::Integer(0) => Ok(ReleaseOutcome::Lost),
    value => Err(RedisLeaseError::UnexpectedScriptValue {
      operation: "release",
      value,
    }),
  }
}
