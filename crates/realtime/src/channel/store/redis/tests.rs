use std::collections::HashSet;

use proptest::prelude::*;
use support::{BootGeneration, NodeId, NodeInstance, timestamp::Timestamp};
use uuid::Uuid;

use crate::{ApplicationId, ChannelKey, ConnectionId};

use super::RedisKeys;

fn instance(boot: u128, started_at: u64) -> NodeInstance {
  NodeInstance::new(
    NodeId::try_new("node-1").unwrap(),
    BootGeneration::from_uuid(Uuid::from_u128(boot)),
    Timestamp::from_millis(started_at),
  )
}

fn connection(value: &str) -> ConnectionId {
  serde_json::from_value(serde_json::json!(value)).unwrap()
}

fn all_keys(keys: &RedisKeys, channel: &ChannelKey, conn: &ConnectionId) -> Vec<String> {
  let app = &channel.application_id;
  let node = instance(1, 1000);
  vec![
    keys.channel_state(channel),
    keys.channel_attachments(channel),
    keys.channel_members(channel),
    keys.channel_shards(channel),
    keys.connection_state(app, conn),
    keys.connection_channels(app, conn),
    keys.connection_members(app, conn),
    keys.connection_operations(app, conn),
    keys.connection_operation_order(app, conn),
    keys.generations(),
    keys.generation_deadlines(),
    keys.generation_connections(&node),
    keys.generation_shards(&node),
    keys.outbox(),
    keys.dirty_channels(),
    keys.occupancy_publications(),
    keys.node_lease(&node.node_id),
    keys.publisher_lease(),
    keys.cleanup_lease(&node),
  ]
}

#[test]
fn persisted_key_layout_is_stable() {
  let keys = RedisKeys::new("webapp", "test").unwrap();
  let app = ApplicationId::new("app");
  let channel = ChannelKey::new(app.clone(), "room");
  let conn = connection("conn");

  assert_eq!(keys.namespace(), "webapp.test.presence.v1");
  assert_eq!(
    keys.channel_state(&channel),
    "webapp.test.presence.v1.app.YXBw.channel.cm9vbQ.state"
  );
  assert_eq!(
    keys.connection_operations(&app, &conn),
    "webapp.test.presence.v1.app.YXBw.connection.Y29ubg.operations"
  );
  assert_eq!(
    keys.generation_connections(&instance(1, 1000)),
    "webapp.test.presence.v1.generation.bm9kZS0x.00000000-0000-0000-0000-000000000001.connections"
  );
  assert_eq!(
    keys.node_lease(&instance(1, 1000).node_id),
    "webapp.test.presence.v1.lease.node.bm9kZS0x"
  );
}

#[test]
fn key_families_and_lease_counters_do_not_overlap() {
  let keys = RedisKeys::new("webapp", "test").unwrap();
  let channel = ChannelKey::new(ApplicationId::new("lease.publisher:fence"), "x:fence");
  let conn = connection("x:fence");
  let mut actual = all_keys(&keys, &channel, &conn);
  let node = instance(1, 1000);
  for lease in [
    keys.node_lease(&node.node_id),
    keys.publisher_lease(),
    keys.cleanup_lease(&node),
  ] {
    // Форма служебного ключа — контракт существующего redis-lease.
    actual.push(format!("{lease}:fence"));
  }

  assert_eq!(actual.iter().collect::<HashSet<_>>().len(), actual.len());
  assert!(
    all_keys(&keys, &channel, &conn)
      .iter()
      .all(|key| !key.contains(':'))
  );
}

#[test]
fn applications_and_environments_have_disjoint_namespaces() {
  let channel = ChannelKey::new(ApplicationId::new("app"), "room");
  let conn = connection("conn");
  let mut seen = HashSet::new();
  for (app, env) in [
    ("webapp", "test"),
    ("other", "test"),
    ("webapp", "production"),
  ] {
    for key in all_keys(&RedisKeys::new(app, env).unwrap(), &channel, &conn) {
      assert!(seen.insert(key));
    }
  }

  for (app, env) in [("a.b", "c"), ("a", "b.c"), ("", "test"), ("app", "")] {
    assert!(RedisKeys::new(app, env).is_err());
  }
}

#[test]
fn boot_changes_generation_keys_but_not_the_node_lease() {
  let keys = RedisKeys::new("webapp", "test").unwrap();
  let first = instance(1, 1000);
  let metadata_changed = instance(1, 2000);
  let restarted = instance(2, 2000);

  for key in [
    RedisKeys::generation_connections,
    RedisKeys::generation_shards,
    RedisKeys::cleanup_lease,
  ] {
    assert_eq!(key(&keys, &first), key(&keys, &metadata_changed));
    assert_ne!(key(&keys, &first), key(&keys, &restarted));
  }
  assert_eq!(
    keys.node_lease(&first.node_id),
    keys.node_lease(&restarted.node_id)
  );

  let mut other_node = first.clone();
  other_node.node_id = NodeId::try_new("node-2").unwrap();
  assert_ne!(
    keys.node_lease(&first.node_id),
    keys.node_lease(&other_node.node_id)
  );
  assert_ne!(
    keys.generation_connections(&first),
    keys.generation_connections(&other_node)
  );
}

#[test]
fn special_names_are_distinct_and_cannot_inject_structure() {
  let keys = RedisKeys::new("webapp", "test").unwrap();
  let names = [
    "", ".", ":fence", "a.b", "a:b", "{room}", "*?[]", "\0", "чат", "é", "e\u{301}",
  ];
  let mut seen = HashSet::new();
  for app in names {
    for channel in names {
      let key = keys.channel_state(&ChannelKey::new(ApplicationId::new(app), channel));
      assert!(
        key
          .bytes()
          .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
      );
      assert!(seen.insert(key));
    }
  }
}

proptest! {
  #[test]
  fn channel_keys_preserve_application_and_channel_identity(
    app_a in any::<String>(), channel_a in any::<String>(),
    app_b in any::<String>(), channel_b in any::<String>(),
  ) {
    let keys = RedisKeys::new("webapp", "test").unwrap();
    let a = ChannelKey::new(ApplicationId::new(app_a), channel_a);
    let b = ChannelKey::new(ApplicationId::new(app_b), channel_b);
    prop_assert_eq!(keys.channel_members(&a) == keys.channel_members(&b), a == b);
  }

  #[test]
  fn connection_keys_preserve_application_and_connection_identity(
    app_a in any::<String>(), conn_a in any::<String>(),
    app_b in any::<String>(), conn_b in any::<String>(),
  ) {
    let keys = RedisKeys::new("webapp", "test").unwrap();
    let a = keys.connection_state(&ApplicationId::new(&app_a), &connection(&conn_a));
    let b = keys.connection_state(&ApplicationId::new(&app_b), &connection(&conn_b));
    prop_assert_eq!(a == b, app_a == app_b && conn_a == conn_b);
  }
}
