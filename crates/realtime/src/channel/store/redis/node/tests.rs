//! Live-тесты NodeLease::claim и renew через настоящий Redis.
//!
//! Запуск: REALTIME_REDIS_TEST_PORT=6379 cargo test -p realtime --lib
//! channel::store::redis::node -- --ignored

use std::time::Duration;

use redis_client::{RedisClient, RedisConfig, ScriptValue};
use redis_lease::{
  LUA_HOLDS_LEASE, LeaseKey, RedisLeaseError, RenewOutcome, fence_key, lease_value,
};
use support::{BootGeneration, NodeId, NodeInstance, timestamp::Timestamp};
use uuid::Uuid;

use super::super::{RedisKeys, protocol::generation, scripts};
use super::{NodeClaimOutcome, NodeLease, NodeLeaseError};

const TTL: Duration = Duration::from_secs(30);

mod attach;

struct Fixture {
  redis: RedisClient,
  keys: RedisKeys,
}

impl Fixture {
  async fn connect() -> Self {
    let config = RedisConfig {
      host: std::env::var("REALTIME_REDIS_TEST_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned()),
      port: std::env::var("REALTIME_REDIS_TEST_PORT")
        .expect("set REALTIME_REDIS_TEST_PORT to run node lease tests"),
      username: std::env::var("REALTIME_REDIS_TEST_USERNAME").ok(),
      password: std::env::var("REALTIME_REDIS_TEST_PASSWORD").ok(),
      ..RedisConfig::default()
    };

    Self {
      redis: RedisClient::connect(&config).await.unwrap(),
      keys: RedisKeys::new("node_claim_test", &Uuid::new_v4().simple().to_string()).unwrap(),
    }
  }

  async fn claim(&self, instance: &NodeInstance, ttl: Duration) -> NodeLease {
    match NodeLease::claim(&self.redis, &self.keys, instance, ttl)
      .await
      .unwrap()
    {
      NodeClaimOutcome::Acquired { lease } => lease,
      other => panic!("expected acquired node lease: {other:?}"),
    }
  }

  async fn state(&self, instance: &NodeInstance) -> Vec<ScriptValue> {
    let lease_key = self.keys.node_lease(&instance.node_id);
    let generations_key = self.keys.generations();
    let deadlines_key = self.keys.generation_deadlines();
    let generation_id = generation(instance);
    let reply = self
      .redis
      .invoke_script(
        "return {redis.call('GET', KEYS[1]), redis.call('HGET', KEYS[2], ARGV[1]), \
       redis.call('ZSCORE', KEYS[3], ARGV[1]), redis.call('PEXPIRETIME', KEYS[1])}",
        &[
          lease_key.as_bytes(),
          generations_key.as_bytes(),
          deadlines_key.as_bytes(),
        ],
        &[generation_id.as_bytes()],
      )
      .await
      .unwrap();

    let ScriptValue::Array(values) = reply else {
      panic!("unexpected node state: {reply:?}");
    };
    values
  }

  async fn assert_registered(&self, lease: &NodeLease) {
    let state = self.state(lease.instance()).await;
    assert_eq!(
      state[0],
      ScriptValue::Bytes(lease_value(lease.token()).into_bytes())
    );
    assert_eq!(
      state[1],
      ScriptValue::Bytes(serde_json::to_vec(lease.instance()).unwrap())
    );
    let ScriptValue::Bytes(deadline) = &state[2] else {
      panic!("missing generation deadline: {state:?}");
    };
    let deadline = std::str::from_utf8(deadline)
      .unwrap()
      .parse::<i64>()
      .unwrap();
    assert_eq!(state[3], ScriptValue::Integer(deadline));
  }

  async fn check_ownership(&self, lease: &NodeLease, token: &str, generation: &str) -> ScriptValue {
    self
      .try_check_ownership(lease, token, generation)
      .await
      .unwrap()
  }

  async fn try_check_ownership(
    &self,
    lease: &NodeLease,
    token: &str,
    generation: &str,
  ) -> redis_client::RedisClientResult<ScriptValue> {
    const SCRIPT: &str = const_format::concatcp!(
      scripts::LUA_CHECK_NODE_LEASE,
      "\nlocal now_ms, rejection = check_node_lease(KEYS[1], KEYS[2], ARGV[1], ARGV[2])\n",
      "if rejection then return rejection end\nreturn {1, now_ms}",
    );
    self
      .redis
      .invoke_script(
        SCRIPT,
        &[
          lease.token().key().as_str().as_bytes(),
          self.keys.generation_deadlines().as_bytes(),
        ],
        &[token.as_bytes(), generation.as_bytes()],
      )
      .await
  }

  async fn cleanup(&self, instance: &NodeInstance) {
    let lease_key = LeaseKey::new(self.keys.node_lease(&instance.node_id)).unwrap();
    let counter_key = fence_key(&lease_key);
    let generations_key = self.keys.generations();
    let deadlines_key = self.keys.generation_deadlines();
    self
      .redis
      .invoke_script(
        "return redis.call('DEL', KEYS[1], KEYS[2], KEYS[3], KEYS[4])",
        &[
          lease_key.as_str().as_bytes(),
          counter_key.as_bytes(),
          generations_key.as_bytes(),
          deadlines_key.as_bytes(),
        ],
        &[],
      )
      .await
      .unwrap();
  }
}

fn instance() -> NodeInstance {
  NodeInstance::new(
    NodeId::try_new("node-1").unwrap(),
    BootGeneration::generate(),
    Timestamp::from_millis(1000),
  )
}

fn assert_ownership_rejection(reply: ScriptValue, code: &str) {
  let ScriptValue::Array(values) = reply else {
    panic!("unexpected ownership response: {reply:?}");
  };
  assert_eq!(values.len(), 3);
  assert_eq!(values[0], ScriptValue::Integer(0));
  assert_eq!(values[1], ScriptValue::Bytes(code.as_bytes().to_vec()));
  assert!(matches!(&values[2], ScriptValue::Bytes(message) if !message.is_empty()));
}

#[ignore = "requires a live Redis (REALTIME_REDIS_TEST_PORT)"]
#[tokio::test]
async fn ownership_check_accepts_live_lease_and_rejects_mismatched_generation() {
  let fx = Fixture::connect().await;
  let node = instance();
  // Fence выше границы точности Lua number должен сравниваться без округления.
  let key = LeaseKey::new(fx.keys.node_lease(&node.node_id)).unwrap();
  fx.redis
    .invoke_script(
      "return redis.call('SET', KEYS[1], '9007199254740992')",
      &[fence_key(&key).as_bytes()],
      &[],
    )
    .await
    .unwrap();
  let lease = fx.claim(&node, TTL).await;
  let token = lease_value(lease.token());
  let generation_id = generation(&node);
  let before = fx.state(&node).await;

  let reply = fx.check_ownership(&lease, &token, &generation_id).await;
  let ScriptValue::Array(values) = reply else {
    panic!("unexpected ownership response: {reply:?}");
  };
  assert_eq!(values.len(), 2);
  assert_eq!(values[0], ScriptValue::Integer(1));
  let ScriptValue::Integer(now_ms) = values[1] else {
    panic!("expected Redis timestamp: {values:?}");
  };
  let ScriptValue::Integer(deadline_ms) = before[3] else {
    panic!("expected lease expiry: {before:?}");
  };
  assert!(now_ms > 0 && now_ms < deadline_ms);

  for (value, owner) in [
    (token.clone(), generation(&instance())),
    (token.clone(), String::new()),
    (format!("lease:{generation_id}:0"), generation_id.clone()),
    (format!("lease:{generation_id}:01"), generation_id.clone()),
    (String::new(), generation_id.clone()),
  ] {
    assert_ownership_rejection(
      fx.check_ownership(&lease, &value, &owner).await,
      "invalid_request",
    );
  }
  assert_ownership_rejection(
    fx.check_ownership(
      &lease,
      &format!("lease:{generation_id}:9007199254740992"),
      &generation_id,
    )
    .await,
    "lease_lost",
  );
  assert_eq!(fx.state(&node).await, before);
  fx.cleanup(&node).await;
}

#[ignore = "requires a live Redis (REALTIME_REDIS_TEST_PORT)"]
#[tokio::test]
async fn ownership_check_rejects_expired_lease_old_token_and_new_boot() {
  let fx = Fixture::connect().await;
  let node = instance();
  let lease = fx.claim(&node, TTL).await;
  let token = lease_value(lease.token());
  let generation_id = generation(&node);

  // Истекает только lease; deadline ещё в будущем, но не заменяет lease.
  fx.redis
    .invoke_script(
      "return redis.call('PEXPIREAT', KEYS[1], 1)",
      &[lease.token().key().as_str().as_bytes()],
      &[],
    )
    .await
    .unwrap();
  let expired = fx.state(&node).await;
  assert_ownership_rejection(
    fx.check_ownership(&lease, &token, &generation_id).await,
    "lease_lost",
  );
  assert_eq!(fx.state(&node).await, expired);

  let next = fx.claim(&node, TTL).await;
  let before = fx.state(&node).await;
  assert_ownership_rejection(
    fx.check_ownership(&lease, &token, &generation_id).await,
    "lease_lost",
  );
  assert_eq!(fx.state(&node).await, before);

  fx.redis
    .invoke_script(
      "return redis.call('PEXPIREAT', KEYS[1], 1)",
      &[next.token().key().as_str().as_bytes()],
      &[],
    )
    .await
    .unwrap();
  let new_node = instance();
  let new_lease = fx.claim(&new_node, TTL).await;
  let before = fx.state(&new_node).await;
  assert_ownership_rejection(
    fx.check_ownership(&next, &lease_value(next.token()), &generation_id)
      .await,
    "lease_lost",
  );
  assert_eq!(fx.state(&new_node).await, before);
  assert!(matches!(
    fx.check_ownership(&new_lease, &lease_value(new_lease.token()), &generation(&new_node)).await,
    ScriptValue::Array(values) if values[0] == ScriptValue::Integer(1)
  ));
  fx.cleanup(&node).await;
}

#[ignore = "requires a live Redis (REALTIME_REDIS_TEST_PORT)"]
#[tokio::test]
async fn ownership_check_requires_valid_live_generation_deadline() {
  let fx = Fixture::connect().await;
  let node = instance();
  let lease = fx.claim(&node, TTL).await;
  let token = lease_value(lease.token());
  let generation_id = generation(&node);
  let deadline_key = fx.keys.generation_deadlines();

  for (score, code) in [
    ("0", "lease_lost"),
    ("-1", "corrupt_state"),
    ("1.5", "corrupt_state"),
    ("+inf", "corrupt_state"),
    ("9007199254740992", "corrupt_state"),
  ] {
    fx.redis
      .invoke_script(
        "return redis.call('ZADD', KEYS[1], ARGV[1], ARGV[2])",
        &[deadline_key.as_bytes()],
        &[score.as_bytes(), generation_id.as_bytes()],
      )
      .await
      .unwrap();
    let before = fx.state(&node).await;
    assert_ownership_rejection(
      fx.check_ownership(&lease, &token, &generation_id).await,
      code,
    );
    assert_eq!(fx.state(&node).await, before);
  }

  // Отсутствующее поколение отклоняется даже при наличии чужого deadline.
  fx.redis.invoke_script(
    "redis.call('ZREM', KEYS[1], ARGV[1]); return redis.call('ZADD', KEYS[1], 9007199254740991, 'other')",
    &[deadline_key.as_bytes()], &[generation_id.as_bytes()],
  ).await.unwrap();
  assert_ownership_rejection(
    fx.check_ownership(&lease, &token, &generation_id).await,
    "lease_lost",
  );
  fx.redis
    .invoke_script(
      "return redis.call('DEL', KEYS[1])",
      &[deadline_key.as_bytes()],
      &[],
    )
    .await
    .unwrap();
  assert_ownership_rejection(
    fx.check_ownership(&lease, &token, &generation_id).await,
    "lease_lost",
  );

  fx.redis
    .invoke_script(
      "return redis.call('SET', KEYS[1], 'wrong-type')",
      &[deadline_key.as_bytes()],
      &[],
    )
    .await
    .unwrap();
  let error = fx
    .try_check_ownership(&lease, &token, &generation_id)
    .await
    .unwrap_err();
  assert_eq!(error.kind(), redis_client::RedisClientErrorKind::Command);
  assert!(error.to_string().contains("WRONGTYPE"));

  fx.redis
    .invoke_script(
      "redis.call('DEL', KEYS[1]); return redis.call('HSET', KEYS[1], 'field', 'value')",
      &[lease.token().key().as_str().as_bytes()],
      &[],
    )
    .await
    .unwrap();
  let error = fx
    .try_check_ownership(&lease, &token, &generation_id)
    .await
    .unwrap_err();
  assert_eq!(error.kind(), redis_client::RedisClientErrorKind::Command);
  assert!(error.to_string().contains("WRONGTYPE"));
  fx.cleanup(&node).await;
}

#[ignore = "requires a live Redis (REALTIME_REDIS_TEST_PORT)"]
#[tokio::test]
async fn claim_registers_instance_and_retry_keeps_the_token() {
  let fx = Fixture::connect().await;
  let node = instance();
  let lease = fx.claim(&node, TTL).await;
  assert_eq!(lease.instance(), &node);
  fx.assert_registered(&lease).await;

  let retry = fx.claim(&node, TTL).await;
  assert_eq!(retry.token(), lease.token());
  fx.assert_registered(&retry).await;

  let before = fx.state(&node).await;
  let other_boot = instance();
  let outcome = NodeLease::claim(&fx.redis, &fx.keys, &other_boot, TTL)
    .await
    .unwrap();
  let NodeClaimOutcome::Held { remaining } = outcome else {
    panic!("another boot must not acquire an active lease: {outcome:?}");
  };
  assert!(remaining > Duration::ZERO && remaining <= TTL);
  assert_eq!(fx.state(&node).await, before);
  let other_state = fx.state(&other_boot).await;
  assert_eq!(other_state[1], ScriptValue::Null);
  assert_eq!(other_state[2], ScriptValue::Null);

  fx.cleanup(&node).await;
}

#[ignore = "requires a live Redis (REALTIME_REDIS_TEST_PORT)"]
#[tokio::test]
async fn claim_preserves_errors_and_rejected_metadata_does_not_change_state() {
  let fx = Fixture::connect().await;
  let node = instance();
  let before = fx.state(&node).await;
  let result = NodeLease::claim(&fx.redis, &fx.keys, &node, Duration::ZERO).await;
  assert!(matches!(
    result,
    Err(NodeLeaseError::Lease(RedisLeaseError::ZeroTtl))
  ));
  assert_eq!(fx.state(&node).await, before);

  let lease = fx.claim(&node, TTL).await;
  let before = fx.state(&node).await;
  let mut conflicting = node.clone();
  conflicting.started_at = Timestamp::from_millis(2000);
  let result = NodeLease::claim(&fx.redis, &fx.keys, &conflicting, TTL).await;
  let Err(NodeLeaseError::Redis(error)) = result else {
    panic!("metadata conflict must remain a Redis error: {result:?}");
  };
  assert!(error.to_string().contains("metadata mismatch"));
  assert_eq!(fx.state(&node).await, before);
  fx.assert_registered(&lease).await;

  fx.cleanup(&node).await;
}

#[ignore = "requires a live Redis (REALTIME_REDIS_TEST_PORT)"]
#[tokio::test]
async fn new_boot_after_expiry_keeps_old_generation_and_rejects_old_token() {
  let fx = Fixture::connect().await;
  let old_node = instance();
  let old_lease = fx.claim(&old_node, Duration::from_millis(50)).await;
  let old_state = fx.state(&old_node).await;
  tokio::time::sleep(Duration::from_millis(150)).await;

  let new_node = instance();
  let new_lease = fx.claim(&new_node, TTL).await;
  assert!(new_lease.token().fence() > old_lease.token().fence());
  fx.assert_registered(&new_lease).await;
  let retained = fx.state(&old_node).await;
  assert_eq!(retained[1], old_state[1]);
  assert_eq!(retained[2], old_state[2]);

  let probe = format!("{LUA_HOLDS_LEASE}\nreturn holds_lease(KEYS[1], ARGV[1]) and 1 or 0");
  let old_token = lease_value(old_lease.token());
  let held = fx
    .redis
    .invoke_script(
      &probe,
      &[old_lease.token().key().as_str().as_bytes()],
      &[old_token.as_bytes()],
    )
    .await
    .unwrap();
  assert_eq!(held, ScriptValue::Integer(0));

  let before = fx.state(&new_node).await;
  assert_eq!(
    old_lease.renew(&fx.redis, &fx.keys, TTL).await.unwrap(),
    RenewOutcome::Lost
  );
  assert_eq!(fx.state(&new_node).await, before);
  assert_eq!(fx.state(&old_node).await, retained);

  fx.cleanup(&new_node).await;
}

#[ignore = "requires a live Redis (REALTIME_REDIS_TEST_PORT)"]
#[tokio::test]
async fn renew_extends_lease_and_deadline_without_changing_token_or_metadata() {
  let fx = Fixture::connect().await;
  let node = instance();
  let lease = fx.claim(&node, Duration::from_secs(5)).await;
  let before = fx.state(&node).await;
  assert_eq!(
    lease.renew(&fx.redis, &fx.keys, TTL).await.unwrap(),
    RenewOutcome::Renewed
  );
  let after = fx.state(&node).await;
  assert_eq!(after[0], before[0]);
  assert_eq!(after[1], before[1]);
  let (ScriptValue::Integer(before_deadline), ScriptValue::Integer(after_deadline)) =
    (&before[3], &after[3])
  else {
    panic!("missing lease deadlines");
  };
  assert!(after_deadline > before_deadline);
  fx.assert_registered(&lease).await;
  fx.cleanup(&node).await;
}

#[ignore = "requires a live Redis (REALTIME_REDIS_TEST_PORT)"]
#[tokio::test]
async fn renew_rejects_expired_token_and_previous_period_of_the_same_generation() {
  let fx = Fixture::connect().await;
  let node = instance();
  let expired = fx.claim(&node, Duration::from_millis(50)).await;
  tokio::time::sleep(Duration::from_millis(150)).await;
  let before = fx.state(&node).await;
  assert_eq!(before[0], ScriptValue::Null);
  assert_eq!(
    expired.renew(&fx.redis, &fx.keys, TTL).await.unwrap(),
    RenewOutcome::Lost
  );
  assert_eq!(fx.state(&node).await, before);

  let current = fx.claim(&node, TTL).await;
  assert_ne!(current.token(), expired.token());
  let before = fx.state(&node).await;
  assert_eq!(
    expired.renew(&fx.redis, &fx.keys, TTL).await.unwrap(),
    RenewOutcome::Lost
  );
  assert_eq!(fx.state(&node).await, before);
  fx.assert_registered(&current).await;
  fx.cleanup(&node).await;
}

#[ignore = "requires a live Redis (REALTIME_REDIS_TEST_PORT)"]
#[tokio::test]
async fn renewal_validation_errors_do_not_extend_the_lease() {
  let fx = Fixture::connect().await;
  let node = instance();
  let lease = fx.claim(&node, TTL).await;
  let before = fx.state(&node).await;

  let other_keys = RedisKeys::new("other_app", "test").unwrap();
  assert!(matches!(
    lease.renew(&fx.redis, &other_keys, TTL).await,
    Err(NodeLeaseError::NamespaceMismatch)
  ));
  assert!(matches!(
    lease.renew(&fx.redis, &fx.keys, Duration::ZERO).await,
    Err(NodeLeaseError::Lease(RedisLeaseError::ZeroTtl))
  ));
  let result = lease
    .renew(&fx.redis, &fx.keys, Duration::from_millis((1 << 53) - 1))
    .await;
  let Err(NodeLeaseError::Redis(error)) = result else {
    panic!("deadline overflow must fail: {result:?}");
  };
  assert!(
    error
      .to_string()
      .contains("deadline exceeds exact integer range")
  );
  assert_eq!(fx.state(&node).await, before);

  let deadline_key = fx.keys.generation_deadlines();
  let token_value = lease_value(lease.token());
  let wrong_generation = generation(&instance());
  let result = fx
    .redis
    .invoke_script(
      scripts::RENEW_NODE,
      &[
        lease.token().key().as_str().as_bytes(),
        deadline_key.as_bytes(),
      ],
      &[
        token_value.as_bytes(),
        b"30000",
        wrong_generation.as_bytes(),
      ],
    )
    .await;
  assert!(
    result
      .unwrap_err()
      .to_string()
      .contains("token does not match generation")
  );
  assert_eq!(fx.state(&node).await, before);

  fx.redis
    .invoke_script(
      "return redis.call('SET', KEYS[1], 'wrong-type')",
      &[deadline_key.as_bytes()],
      &[],
    )
    .await
    .unwrap();
  let result = lease.renew(&fx.redis, &fx.keys, TTL).await;
  let Err(NodeLeaseError::Redis(error)) = result else {
    panic!("wrong deadline type must fail: {result:?}");
  };
  assert!(error.to_string().contains("must be a zset"));
  let after = fx.redis.invoke_script(
    "return {redis.call('GET', KEYS[1]), redis.call('PEXPIRETIME', KEYS[1]), redis.call('GET', KEYS[2])}",
    &[lease.token().key().as_str().as_bytes(), deadline_key.as_bytes()], &[],
  ).await.unwrap();
  assert_eq!(
    after,
    ScriptValue::Array(vec![
      before[0].clone(),
      before[3].clone(),
      ScriptValue::Bytes(b"wrong-type".to_vec()),
    ])
  );
  fx.cleanup(&node).await;
}

#[ignore = "requires a live Redis (REALTIME_REDIS_TEST_PORT)"]
#[tokio::test]
async fn renew_can_retry_with_the_same_token_after_a_committed_result_is_unobserved() {
  use std::io::Write;
  use std::net::TcpStream;

  // Подавляем ответ отдельного соединения: скрипт выполнится, но клиент не
  // получит подтверждение. Состояние проверяем независимым соединением.
  fn send(stream: &mut TcpStream, args: &[&[u8]]) {
    let mut command = format!("*{}\r\n", args.len()).into_bytes();
    for arg in args {
      command.extend_from_slice(format!("${}\r\n", arg.len()).as_bytes());
      command.extend_from_slice(arg);
      command.extend_from_slice(b"\r\n");
    }
    stream.write_all(&command).unwrap();
  }

  let fx = Fixture::connect().await;
  let node = instance();
  let lease = fx.claim(&node, Duration::from_secs(5)).await;
  let before = fx.state(&node).await;
  let host = std::env::var("REALTIME_REDIS_TEST_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
  let port = std::env::var("REALTIME_REDIS_TEST_PORT")
    .unwrap()
    .parse::<u16>()
    .unwrap();
  let mut connection = TcpStream::connect((host.as_str(), port)).unwrap();
  connection
    .set_write_timeout(Some(Duration::from_secs(3)))
    .unwrap();
  if let Ok(password) = std::env::var("REALTIME_REDIS_TEST_PASSWORD") {
    if let Ok(username) = std::env::var("REALTIME_REDIS_TEST_USERNAME") {
      send(
        &mut connection,
        &[b"AUTH", username.as_bytes(), password.as_bytes()],
      );
    } else {
      send(&mut connection, &[b"AUTH", password.as_bytes()]);
    }
  }
  send(&mut connection, &[b"CLIENT", b"REPLY", b"OFF"]);
  let deadline_key = fx.keys.generation_deadlines();
  let token_value = lease_value(lease.token());
  let generation_id = generation(&node);
  send(
    &mut connection,
    &[
      b"EVAL",
      scripts::RENEW_NODE.as_bytes(),
      b"2",
      lease.token().key().as_str().as_bytes(),
      deadline_key.as_bytes(),
      token_value.as_bytes(),
      b"30000",
      generation_id.as_bytes(),
    ],
  );
  tokio::time::timeout(Duration::from_secs(3), async {
    loop {
      if fx.state(&node).await[3] != before[3] {
        break;
      }
      tokio::time::sleep(Duration::from_millis(10)).await;
    }
  })
  .await
  .expect("renewal must commit even without a reply");
  drop(connection);
  fx.assert_registered(&lease).await;

  assert_eq!(
    lease.renew(&fx.redis, &fx.keys, TTL).await.unwrap(),
    RenewOutcome::Renewed
  );
  let after = fx.state(&node).await;
  assert_eq!(after[0], before[0]);
  assert_eq!(after[1], before[1]);
  fx.assert_registered(&lease).await;
  fx.cleanup(&node).await;
}
