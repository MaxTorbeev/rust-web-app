use super::*;

fn event_id(value: &str) -> Uuid {
  Uuid::parse_str(value).expect("test event id must be valid")
}

#[test]
fn builds_full_key_from_prefix_scope_and_event_id() {
  let key = DedupKey::new("node-7", event_id("018f47a3-7c83-7e80-a43a-9d07432f2e91"));

  assert_eq!(
    redis_key("mxt.production.event-bus.v1.dedup", &key),
    "mxt.production.event-bus.v1.dedup.node-7.018f47a3-7c83-7e80-a43a-9d07432f2e91"
  );
}

#[test]
fn key_identity_includes_scope_and_event_id() {
  let first_event_id = event_id("018f47a3-7c83-7e80-a43a-9d07432f2e91");
  let second_event_id = event_id("018f47a3-7c83-7e80-a43a-9d07432f2e92");

  let first = redis_key("prefix", &DedupKey::new("node-1", first_event_id));
  let another_scope = redis_key("prefix", &DedupKey::new("node-2", first_event_id));
  let another_event = redis_key("prefix", &DedupKey::new("node-1", second_event_id));

  assert_ne!(first, another_scope);
  assert_ne!(first, another_event);
}

#[test]
fn encodes_lease_token() {
  let token = event_id("018f47a3-7c83-7e80-a43a-9d07432f2e91");

  assert_eq!(
    lease_value(token),
    "lease:018f47a3-7c83-7e80-a43a-9d07432f2e91"
  );
}

#[test]
fn rounds_positive_ttl_up_to_redis_milliseconds() {
  for (ttl, expected) in [
    (Duration::from_nanos(1), 1),
    (Duration::from_nanos(999_999), 1),
    (Duration::from_millis(1), 1),
    (Duration::from_nanos(1_000_001), 2),
    (Duration::from_millis(5_000), 5_000),
  ] {
    assert_eq!(redis_ttl_milliseconds(ttl).unwrap(), expected);
  }
}

#[test]
fn rejects_zero_and_overflowing_ttl() {
  assert!(matches!(
    redis_ttl_milliseconds(Duration::ZERO),
    Err(RedisDedupStoreError::ZeroTtl)
  ));

  let maximum = Duration::from_millis(i64::MAX as u64);
  assert_eq!(redis_ttl_milliseconds(maximum).unwrap(), i64::MAX);

  let overflow = Duration::from_millis(i64::MAX as u64 + 1);
  assert!(matches!(
    redis_ttl_milliseconds(overflow),
    Err(RedisDedupStoreError::TtlOverflow { .. })
  ));
}

#[test]
fn decodes_claim_outcomes() {
  assert_eq!(
    decode_claim_value(ScriptValue::Array(vec![ScriptValue::Integer(1)])).unwrap(),
    ClaimScriptOutcome::Acquired
  );
  assert_eq!(
    decode_claim_value(ScriptValue::Array(vec![ScriptValue::Integer(2)])).unwrap(),
    ClaimScriptOutcome::Completed
  );
  assert_eq!(
    decode_claim_value(ScriptValue::Array(vec![
      ScriptValue::Integer(3),
      ScriptValue::Integer(0),
    ]))
    .unwrap(),
    ClaimScriptOutcome::InProgress {
      retry_after: Duration::ZERO,
    }
  );
  assert_eq!(
    decode_claim_value(ScriptValue::Array(vec![
      ScriptValue::Integer(3),
      ScriptValue::Integer(42),
    ]))
    .unwrap(),
    ClaimScriptOutcome::InProgress {
      retry_after: Duration::from_millis(42),
    }
  );
}

#[test]
fn rejects_unexpected_claim_values() {
  for value in [
    ScriptValue::Null,
    ScriptValue::Integer(1),
    ScriptValue::Array(vec![]),
    ScriptValue::Array(vec![ScriptValue::Integer(1), ScriptValue::Integer(0)]),
    ScriptValue::Array(vec![ScriptValue::Integer(2), ScriptValue::Integer(0)]),
    ScriptValue::Array(vec![ScriptValue::Integer(3)]),
    ScriptValue::Array(vec![ScriptValue::Integer(4)]),
    ScriptValue::Array(vec![ScriptValue::Integer(3), ScriptValue::Integer(-1)]),
    ScriptValue::Array(vec![
      ScriptValue::Integer(3),
      ScriptValue::Bytes(b"1".to_vec()),
    ]),
    ScriptValue::Array(vec![
      ScriptValue::Integer(3),
      ScriptValue::Integer(1),
      ScriptValue::Integer(2),
    ]),
  ] {
    assert!(matches!(
      decode_claim_value(value),
      Err(RedisDedupStoreError::UnexpectedClaimValue { .. })
    ));
  }
}

#[test]
fn decodes_lease_transition_values() {
  assert!(decode_transition_value("complete", ScriptValue::Integer(1)).unwrap());
  assert!(!decode_transition_value("release", ScriptValue::Integer(0)).unwrap());

  for value in [
    ScriptValue::Null,
    ScriptValue::Integer(-1),
    ScriptValue::Integer(2),
    ScriptValue::Bytes(b"1".to_vec()),
    ScriptValue::Array(vec![ScriptValue::Integer(1)]),
  ] {
    assert!(matches!(
      decode_transition_value("complete", value),
      Err(RedisDedupStoreError::UnexpectedTransitionValue {
        operation: "complete",
        ..
      })
    ));
  }
}
