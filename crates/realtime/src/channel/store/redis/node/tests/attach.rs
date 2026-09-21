//! Проверки attach используют тот же live Redis fixture и настоящий node lease.

use redis_client::{RedisClientErrorKind, RedisClientResult, ScriptValue};
use serde_json::{Value, json};

use crate::{ApplicationId, Attachment, AttachmentTracking, ChannelKey, ChannelMode, ConnectionId};

use super::super::super::protocol::segment;
use super::{
  Fixture, NodeLease, TTL, assert_ownership_rejection, generation, instance, lease_value, scripts,
};

struct AttachFixture {
  node: Fixture,
  lease: NodeLease,
  attachment: Attachment,
  connection_key: String,
  attachments_key: String,
}

impl AttachFixture {
  async fn new() -> Self {
    let node = Fixture::connect().await;
    let lease = node.claim(&instance(), TTL).await;
    let channel = ChannelKey::new(ApplicationId::new("app"), "room");
    let connection: ConnectionId = serde_json::from_value(json!("connection-1")).unwrap();
    Self {
      connection_key: node
        .keys
        .connection_state(&channel.application_id, &connection),
      attachments_key: node.keys.channel_attachments(&channel),
      attachment: Attachment {
        connection_id: connection,
        node_instance: lease.instance().clone(),
        accounting: AttachmentTracking::Individual,
        effective_modes: vec![ChannelMode::Subscribe, ChannelMode::Presence],
        occupancy: None,
      },
      node,
      lease,
    }
  }

  async fn eval(&self, script: &str, args: &[&[u8]]) -> ScriptValue {
    self
      .node
      .redis
      .invoke_script(
        script,
        &[
          self.connection_key.as_bytes(),
          self.attachments_key.as_bytes(),
        ],
        args,
      )
      .await
      .unwrap()
  }

  async fn open(&self) {
    self
      .eval(
        "return redis.call('HSET', KEYS[1], 'status', 'open', 'generation', ARGV[1])",
        &[generation(self.lease.instance()).as_bytes()],
      )
      .await;
  }

  async fn save(&self, json: &str) {
    self
      .eval(
        "return redis.call('HSET', KEYS[2], ARGV[1], ARGV[2])",
        &[
          segment(self.attachment.connection_id.as_str()).as_bytes(),
          json.as_bytes(),
        ],
      )
      .await;
  }

  async fn check(&self) -> RedisClientResult<ScriptValue> {
    // Только тестовый вход для проверки helper без commit transition.
    const SCRIPT: &str = const_format::concatcp!(
      scripts::LUA_CHECK_NODE_LEASE,
      "\n",
      scripts::LUA_CHECK_ATTACH_STATE,
      "\nlocal now, rejection = check_node_lease(KEYS[1], KEYS[2], ARGV[1], ARGV[2])\n",
      "if rejection then return rejection end\n",
      "local connection_exists = redis.call('TYPE', KEYS[3]).ok ~= 'none'\n",
      "local previous, failure = check_attach_state(KEYS[3], KEYS[4], ARGV[3], ARGV[2], cjson.decode(ARGV[4]), connection_exists)\n",
      "if failure then return failure end\nreturn {1, previous and 1 or 0}",
    );
    let before = self
      .eval(
        "return {redis.call('DUMP', KEYS[1]), redis.call('DUMP', KEYS[2])}",
        &[],
      )
      .await;
    let lease_before = self.node.state(self.lease.instance()).await;
    let reply = self
      .node
      .redis
      .invoke_script(
        SCRIPT,
        &[
          self.lease.token().key().as_str().as_bytes(),
          self.node.keys.generation_deadlines().as_bytes(),
          self.connection_key.as_bytes(),
          self.attachments_key.as_bytes(),
        ],
        &[
          lease_value(self.lease.token()).as_bytes(),
          generation(self.lease.instance()).as_bytes(),
          segment(self.attachment.connection_id.as_str()).as_bytes(),
          serde_json::to_string(&self.attachment).unwrap().as_bytes(),
        ],
      )
      .await;
    assert_eq!(
      self
        .eval(
          "return {redis.call('DUMP', KEYS[1]), redis.call('DUMP', KEYS[2])}",
          &[]
        )
        .await,
      before
    );
    assert_eq!(self.node.state(self.lease.instance()).await, lease_before);
    reply
  }

  async fn cleanup(&self) {
    self
      .eval("return redis.call('DEL', KEYS[1], KEYS[2])", &[])
      .await;
    self.node.cleanup(self.lease.instance()).await;
  }
}

#[ignore = "requires a live Redis (REALTIME_REDIS_TEST_PORT)"]
#[tokio::test]
async fn attach_check_accepts_new_connection_and_same_generation_attachment() {
  let fx = AttachFixture::new().await;
  let absent = ScriptValue::Array(vec![ScriptValue::Integer(1), ScriptValue::Integer(0)]);
  assert_eq!(fx.check().await.unwrap(), absent);
  fx.open().await;
  // Живое соединение может впервые присоединяться к этому каналу.
  assert_eq!(fx.check().await.unwrap(), absent);
  let mut saved = serde_json::to_value(&fx.attachment).unwrap();
  saved["nodeInstance"]["startedAt"] = json!(2000);
  fx.save(&saved.to_string()).await;
  assert_eq!(
    fx.check().await.unwrap(),
    ScriptValue::Array(vec![ScriptValue::Integer(1), ScriptValue::Integer(1)])
  );
  fx.cleanup().await;
}

#[ignore = "requires a live Redis (REALTIME_REDIS_TEST_PORT)"]
#[tokio::test]
async fn attach_check_rejects_closed_foreign_and_incomplete_connection_state() {
  let fx = AttachFixture::new().await;
  for (status, owner, code) in [
    (
      Some("closed"),
      Some(generation(fx.lease.instance())),
      "connection_closed",
    ),
    (
      Some("open"),
      Some(generation(&instance())),
      "generation_mismatch",
    ),
    (
      Some("unknown"),
      Some(generation(fx.lease.instance())),
      "corrupt_state",
    ),
    (Some("open"), None, "corrupt_state"),
    (None, Some(generation(fx.lease.instance())), "corrupt_state"),
    (None, None, "corrupt_state"),
  ] {
    fx.eval("redis.call('DEL', KEYS[1]); return redis.call('HSET', KEYS[1], 'highest_serial', '00000000000000000001')", &[]).await;
    if let Some(status) = status {
      fx.eval(
        "return redis.call('HSET', KEYS[1], 'status', ARGV[1])",
        &[status.as_bytes()],
      )
      .await;
    }
    if let Some(owner) = owner {
      fx.eval(
        "return redis.call('HSET', KEYS[1], 'generation', ARGV[1])",
        &[owner.as_bytes()],
      )
      .await;
    }
    assert_ownership_rejection(fx.check().await.unwrap(), code);
  }
  fx.cleanup().await;
}

#[ignore = "requires a live Redis (REALTIME_REDIS_TEST_PORT)"]
#[tokio::test]
async fn attach_check_rejects_foreign_or_malformed_saved_attachment() {
  let fx = AttachFixture::new().await;
  let original = serde_json::to_value(&fx.attachment).unwrap();
  fx.save(&original.to_string()).await;
  assert_ownership_rejection(fx.check().await.unwrap(), "corrupt_state"); // Нет connection state.
  fx.open().await;

  for (pointer, value, code) in [
    ("/connectionId", json!("other"), "corrupt_state"),
    (
      "/nodeInstance/nodeId",
      json!("other-node"),
      "generation_mismatch",
    ),
    (
      "/nodeInstance/bootGeneration",
      json!(instance().boot_generation),
      "generation_mismatch",
    ),
    ("/nodeInstance", Value::Null, "corrupt_state"),
    ("/accounting", json!("aggregated"), "corrupt_state"),
    ("/effectiveModes", json!([]), "corrupt_state"),
    ("/effectiveModes", json!(["unknown"]), "corrupt_state"),
    (
      "/effectiveModes",
      json!({"1": "subscribe"}),
      "corrupt_state",
    ),
    ("/effectiveModes", json!("subscribe"), "corrupt_state"),
  ] {
    let mut saved = original.clone();
    *saved.pointer_mut(pointer).unwrap() = value;
    fx.save(&saved.to_string()).await;
    assert_ownership_rejection(fx.check().await.unwrap(), code);
  }
  for raw in ["{", "null", "[]", "{}", "1", "\"text\""] {
    fx.save(raw).await;
    assert_ownership_rejection(fx.check().await.unwrap(), "corrupt_state");
  }
  fx.cleanup().await;
}

#[ignore = "requires a live Redis (REALTIME_REDIS_TEST_PORT)"]
#[tokio::test]
async fn attach_check_rejects_wrong_redis_types_and_checks_lease_first() {
  let fx = AttachFixture::new().await;
  fx.eval("return redis.call('SET', KEYS[1], 'wrong-type')", &[])
    .await;
  let error = fx.check().await.unwrap_err();
  assert_eq!(error.kind(), RedisClientErrorKind::Command);
  assert!(error.to_string().contains("WRONGTYPE"));
  fx.eval("return redis.call('DEL', KEYS[1])", &[]).await;
  fx.open().await;
  fx.eval("return redis.call('SET', KEYS[2], 'wrong-type')", &[])
    .await;
  let error = fx.check().await.unwrap_err();
  assert!(error.to_string().contains("WRONGTYPE"));

  fx.node
    .redis
    .invoke_script(
      "return redis.call('PEXPIREAT', KEYS[1], 1)",
      &[fx.lease.token().key().as_str().as_bytes()],
      &[],
    )
    .await
    .unwrap();
  // Lease проверяется раньше повреждённого attachment HASH.
  assert_ownership_rejection(fx.check().await.unwrap(), "lease_lost");
  fx.cleanup().await;
}
