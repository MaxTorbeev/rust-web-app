use std::time::Duration;

use redis_client::ScriptValue;

use crate::error::RedisLeaseError;
use crate::identity::{LeaseKey, LeaseOwner, LeaseToken};
use crate::outcome::{AcquireOutcome, Fence, ReleaseOutcome, RenewOutcome};
use crate::protocol::{
  decode_acquire, decode_release, decode_renew, fence_key, lease_value, owner_value,
  redis_ttl_milliseconds,
};

fn key() -> LeaseKey {
  LeaseKey::new("app.local.presence.v1.node.node-1").unwrap()
}

fn owner() -> LeaseOwner {
  LeaseOwner::new("node-1:293a2951-5ba0-482c-91c7-0a0c72a5ce4b").unwrap()
}

#[test]
fn lease_value_format_is_the_public_contract() {
  // Формат читают Lua-скрипты других крейтов: любое изменение здесь — изменение
  // протокола, а не деталь реализации. Fence — последний сегмент, владелец
  // может содержать `:`.
  let token = LeaseToken::new(key(), owner(), Fence::new(7));

  assert_eq!(
    lease_value(&token),
    "lease:node-1:293a2951-5ba0-482c-91c7-0a0c72a5ce4b:7"
  );
}

#[test]
fn owner_value_is_the_lease_value_without_fence() {
  // `acquire.lua` получает значение владельца и дописывает `:<fence>`; то, что
  // он запишет, обязано совпасть с `lease_value` token-а, иначе renew/release
  // никогда не найдут собственный lease.
  let token = LeaseToken::new(key(), owner(), Fence::new(7));

  assert_eq!(
    owner_value(&owner()),
    "lease:node-1:293a2951-5ba0-482c-91c7-0a0c72a5ce4b"
  );
  assert_eq!(
    lease_value(&token),
    format!("{}:{}", owner_value(&owner()), token.fence().get())
  );
}

#[test]
fn tokens_of_different_periods_have_different_values() {
  let first = LeaseToken::new(key(), owner(), Fence::new(1));
  let second = LeaseToken::new(key(), owner(), Fence::new(2));

  assert_ne!(first, second);
  assert_ne!(lease_value(&first), lease_value(&second));
}

#[test]
fn fence_key_lives_next_to_the_lease_key() {
  let key = LeaseKey::new("app.local.presence.v1.node.node-1").unwrap();

  assert_eq!(fence_key(&key), "app.local.presence.v1.node.node-1:fence");
}

#[test]
fn ttl_is_rounded_up_to_whole_milliseconds_and_never_zero() {
  assert!(matches!(
    redis_ttl_milliseconds(Duration::ZERO),
    Err(RedisLeaseError::ZeroTtl)
  ));
  assert_eq!(redis_ttl_milliseconds(Duration::from_nanos(1)).unwrap(), 1);
  assert_eq!(
    redis_ttl_milliseconds(Duration::from_micros(1_500)).unwrap(),
    2
  );
  assert_eq!(
    redis_ttl_milliseconds(Duration::from_secs(15)).unwrap(),
    15_000
  );
  assert!(matches!(
    redis_ttl_milliseconds(Duration::MAX),
    Err(RedisLeaseError::TtlOverflow { .. })
  ));
}

#[test]
fn acquire_reply_is_decoded() {
  // Token собирается из ключа и владельца запроса и fence из ответа.
  assert_eq!(
    decode_acquire(
      ScriptValue::Array(vec![ScriptValue::Integer(1), ScriptValue::Integer(7)]),
      &key(),
      &owner(),
    )
    .unwrap(),
    AcquireOutcome::Acquired {
      token: LeaseToken::new(key(), owner(), Fence::new(7)),
    },
  );
  assert_eq!(
    decode_acquire(
      ScriptValue::Array(vec![ScriptValue::Integer(2), ScriptValue::Integer(1_500)]),
      &key(),
      &owner(),
    )
    .unwrap(),
    AcquireOutcome::Held {
      remaining: Duration::from_millis(1_500)
    },
  );

  for bad in [
    ScriptValue::Integer(1),
    ScriptValue::Array(vec![ScriptValue::Integer(3), ScriptValue::Integer(1)]),
    ScriptValue::Array(vec![ScriptValue::Integer(1), ScriptValue::Integer(-1)]),
    ScriptValue::Null,
  ] {
    assert!(matches!(
      decode_acquire(bad, &key(), &owner()),
      Err(RedisLeaseError::UnexpectedScriptValue {
        operation: "acquire",
        ..
      })
    ));
  }
}

#[test]
fn renew_and_release_replies_are_decoded() {
  assert_eq!(
    decode_renew(ScriptValue::Integer(1)).unwrap(),
    RenewOutcome::Renewed
  );
  assert_eq!(
    decode_renew(ScriptValue::Integer(0)).unwrap(),
    RenewOutcome::Lost
  );
  assert!(matches!(
    decode_renew(ScriptValue::Integer(2)),
    Err(RedisLeaseError::UnexpectedScriptValue {
      operation: "renew",
      ..
    })
  ));

  assert_eq!(
    decode_release(ScriptValue::Integer(1)).unwrap(),
    ReleaseOutcome::Released
  );
  assert_eq!(
    decode_release(ScriptValue::Integer(0)).unwrap(),
    ReleaseOutcome::Lost
  );
  assert!(matches!(
    decode_release(ScriptValue::Bytes(b"OK".to_vec())),
    Err(RedisLeaseError::UnexpectedScriptValue {
      operation: "release",
      ..
    })
  ));
}
