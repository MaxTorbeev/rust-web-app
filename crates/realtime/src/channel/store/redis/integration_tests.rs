//! REALTIME_REDIS_TEST_PORT=16389 cargo test -p realtime --lib redis::integration_tests -- --ignored
use super::{
  NodeClaimOutcome, NodeLease, RedisChannelStore, RedisKeys, RedisPresenceRuntime,
  protocol::{generation, node_lease_owner},
};
use crate::*;
use event_bus::{
  DeliveryClass, EventBus, EventBusError, EventDispatcher, EventMessage, EventPublishFuture,
  EventPublisher,
};
use redis_client::{RedisClient, RedisConfig, ScriptValue};
use redis_lease::{AcquireOutcome, LeaseKey, RedisLease};
use serde_json::json;
use std::{
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
  },
  time::Duration,
};
use support::{BootGeneration, NodeId, NodeInstance, fresh_uuid, timestamp::Timestamp};

struct Fixture {
  redis: Arc<RedisClient>,
  keys: RedisKeys,
}
impl Fixture {
  async fn new() -> Self {
    let redis = Arc::new(
      RedisClient::connect(&RedisConfig {
        host: "127.0.0.1".into(),
        port: std::env::var("REALTIME_REDIS_TEST_PORT").expect("test Redis port"),
        ..RedisConfig::default()
      })
      .await
      .unwrap(),
    );
    Self {
      redis,
      keys: RedisKeys::new("presence_integration", &fresh_uuid().simple().to_string()).unwrap(),
    }
  }
  async fn node(&self, name: &str) -> Arc<RedisChannelStore> {
    let instance = NodeInstance::new(
      NodeId::try_new(name).unwrap(),
      BootGeneration::generate(),
      Timestamp::now(),
    );
    let NodeClaimOutcome::Acquired { lease } =
      NodeLease::claim(&self.redis, &self.keys, &instance, Duration::from_secs(30))
        .await
        .unwrap()
    else {
      panic!("node held")
    };
    Arc::new(
      RedisChannelStore::new(
        self.redis.clone(),
        self.keys.clone(),
        Arc::new(lease),
        PresenceLedgerPolicy {
          capacity: 3,
          retention: Duration::from_secs(1),
        },
      )
      .unwrap(),
    )
  }
  async fn expire(&self, store: &RedisChannelStore) {
    self
      .redis
      .invoke_script(
        "redis.call('DEL', KEYS[1]); return redis.call('ZADD', KEYS[2], 0, ARGV[1])",
        &[
          self
            .keys
            .node_lease(&store.node_instance().node_id)
            .as_bytes(),
          self.keys.generation_deadlines().as_bytes(),
        ],
        &[generation(store.node_instance()).as_bytes()],
      )
      .await
      .unwrap();
  }
  async fn outbox_len(&self) -> i64 {
    let ScriptValue::Integer(n) = self
      .redis
      .invoke_script(
        "return redis.call('XLEN', KEYS[1])",
        &[self.keys.outbox().as_bytes()],
        &[],
      )
      .await
      .unwrap()
    else {
      panic!()
    };
    n
  }
}
fn actor(store: &RedisChannelStore) -> ConnectionActor {
  ConnectionActor {
    application_id: ApplicationId::new("test"),
    connection_id: ConnectionId::generate(),
    node_instance: store.node_instance().clone(),
  }
}
fn room() -> ChannelKey {
  ChannelKey::new(ApplicationId::new("test"), "room")
}
async fn attach(store: &RedisChannelStore, actor: &ConnectionActor) -> ChannelAttachOutcome {
  AttachmentStore::attach_and_snapshot(
    store,
    AttachCommand {
      channel: room(),
      actor: actor.clone(),
      accounting: AttachmentTracking::Individual,
      effective_modes: ChannelMode::ALL.to_vec(),
      occupancy: None,
      request_time: Timestamp::from_millis(1),
      event_id: fresh_uuid(),
    },
  )
  .await
  .unwrap()
}
fn enter(actor: &ConnectionActor, serial: u64) -> PresenceBatchCommand {
  PresenceBatchCommand {
    channel: room(),
    actor: PresenceActor {
      connection_actor: actor.clone(),
      client_id_policy: PresenceClientIdPolicy::Any,
    },
    request_fingerprint: format!("enter-{serial}"),
    items: vec![PresenceBatchItem {
      action: PresenceMutationAction::Enter,
      client_id: Some("alice".into()),
      data: Some(json!({"large": 18446744073709551615u64})),
    }],
    msg_serial: serial,
    request_time: Timestamp::from_millis(1),
    event_id: fresh_uuid(),
  }
}
fn committed(receipt: &PresenceMutationReceipt) -> &CommittedPresenceEvent {
  let PresenceMutationOutcome::Committed(CommittedChannelTransition::Changed(event)) =
    &receipt.outcome
  else {
    panic!("{receipt:?}")
  };
  event
}
fn disconnect(actor: &ConnectionActor) -> DisconnectConnectionCommand {
  DisconnectConnectionCommand {
    actor: actor.clone(),
    request_time: Timestamp::now(),
    event_id: fresh_uuid(),
  }
}

#[tokio::test]
#[ignore = "requires an isolated Redis"]
async fn shared_state_replay_disconnect_and_generation_cleanup() {
  let f = Fixture::new().await;
  let old = f.node("node-a").await;
  let reaper = f.node("node-b").await;
  let owner = actor(&old);
  attach(&old, &owner).await;
  let command = enter(&owner, u64::MAX);
  let receipt = PresenceStore::apply_presence(old.as_ref(), command.clone())
    .await
    .unwrap();
  assert!(committed(&receipt).change().occurred_at.as_millis() > 1);
  let snapshot = PresenceStore::snapshot(reaper.as_ref(), room())
    .await
    .unwrap();
  assert_eq!(snapshot.members.len(), 1);
  assert_eq!(
    snapshot.members[0].data,
    Some(json!({"large": 18446744073709551615u64}))
  );
  let replay = old.apply_presence(command.clone()).await.unwrap();
  assert!(replay.replayed);
  assert_eq!(
    committed(&receipt).event_id(),
    committed(&replay).event_id()
  );
  assert_eq!(f.outbox_len().await, 2);
  let mut conflict = command;
  conflict.request_fingerprint = "different".into();
  assert!(matches!(
    old.apply_presence(conflict).await.unwrap().outcome,
    PresenceMutationOutcome::Rejected(_)
  ));
  assert!(old.expired_generations(0).await.unwrap().is_empty());
  f.expire(&old).await;
  let new = f.node("node-a").await;
  let new_actor = actor(&new);
  attach(&new, &new_actor).await;
  new.apply_presence(enter(&new_actor, 1)).await.unwrap();
  assert!(old.apply_presence(enter(&owner, 2)).await.is_err());
  assert_eq!(
    reaper.expired_generations(1).await.unwrap(),
    vec![old.node_instance().clone()]
  );
  let leases = RedisLease::new(f.redis.clone());
  let key = LeaseKey::new(f.keys.cleanup_lease(old.node_instance())).unwrap();
  let AcquireOutcome::Acquired { token } = leases
    .acquire(
      &key,
      &node_lease_owner(reaper.node_instance()).unwrap(),
      Duration::from_secs(30),
    )
    .await
    .unwrap()
  else {
    panic!()
  };
  assert!(
    !reaper
      .reap_generation_batch(old.node_instance(), &token, 1)
      .await
      .unwrap()
  );
  let after = new.snapshot(room()).await.unwrap();
  assert_eq!(after.members.len(), 1);
  assert_eq!(after.members[0].connection_id, new_actor.connection_id);
  let count = f.outbox_len().await;
  assert!(
    reaper
      .reap_connection(disconnect(&owner), &token)
      .await
      .unwrap()
      .is_empty()
  );
  assert_eq!(f.outbox_len().await, count);
  assert!(
    reaper
      .reap_generation_batch(old.node_instance(), &token, 1)
      .await
      .unwrap()
  );
  assert!(reaper.expired_generations(10).await.unwrap().is_empty());
  new.disconnect(disconnect(&new_actor)).await.unwrap();
  assert!(
    new
      .disconnect(disconnect(&new_actor))
      .await
      .unwrap()
      .is_empty()
  );
  assert_eq!(new.snapshot(room()).await.unwrap().occupancy.connections, 0);
  leases.release(&token).await.unwrap();
}

#[derive(Default)]
struct RecordingPublisher {
  events: Mutex<Vec<EventMessage>>,
  fail_once: AtomicBool,
}
impl EventPublisher for RecordingPublisher {
  fn publish<'a>(&'a self, message: &'a EventMessage, _: DeliveryClass) -> EventPublishFuture<'a> {
    Box::pin(async move {
      self.events.lock().unwrap().push(message.clone());
      if self.fail_once.swap(false, Ordering::SeqCst) {
        return Err(EventBusError::publisher(std::io::Error::other(
          "lost publish ACK",
        )));
      }
      Ok(())
    })
  }
}
#[tokio::test]
#[ignore = "requires an isolated Redis"]
async fn outbox_retries_the_committed_event_and_rejects_stale_publisher() {
  let f = Fixture::new().await;
  let store = f.node("publisher").await;
  attach(&store, &actor(&store)).await;
  let publisher = Arc::new(RecordingPublisher::default());
  publisher.fail_once.store(true, Ordering::SeqCst);
  let bus =
    EventBus::with_distributed_publisher(Arc::new(EventDispatcher::new()), publisher.clone());
  let leases = RedisLease::new(f.redis.clone());
  let key = LeaseKey::new(f.keys.publisher_lease()).unwrap();
  let owner = node_lease_owner(store.node_instance()).unwrap();
  let AcquireOutcome::Acquired { token } = leases
    .acquire(&key, &owner, Duration::from_secs(30))
    .await
    .unwrap()
  else {
    panic!()
  };
  assert!(store.publish_outbox_batch(&token, &bus).await.is_err());
  assert_eq!(f.outbox_len().await, 1);
  assert_eq!(store.publish_outbox_batch(&token, &bus).await.unwrap(), 1);
  assert_eq!(f.outbox_len().await, 0);
  let messages = publisher.events.lock().unwrap();
  assert_eq!(messages[0], messages[1]);
  drop(messages);
  leases.release(&token).await.unwrap();
  let AcquireOutcome::Acquired { token: next } = leases
    .acquire(&key, &owner, Duration::from_secs(30))
    .await
    .unwrap()
  else {
    panic!()
  };
  assert!(store.publish_outbox_batch(&token, &bus).await.is_err());
  assert_eq!(store.publish_outbox_batch(&next, &bus).await.unwrap(), 0);
}

#[tokio::test]
#[ignore = "requires an isolated Redis"]
async fn pending_attach_dedup_and_revision_gap_use_authoritative_snapshot() {
  let f = Fixture::new().await;
  let store = f.node("projection").await;
  let owner = actor(&store);
  let initial = attach(&store, &owner).await.snapshot;
  let router = ChannelRouter::new();
  let (shutdown, _listener) = shutdown_channel();
  let (tx, mut rx) = tokio::sync::mpsc::channel(16);
  let sender = OutboundSender::new(tx, shutdown);
  router
    .begin_attach(
      "room",
      owner.connection_id.clone(),
      sender,
      vec![ChannelMode::PresenceSubscribe],
    )
    .await;
  let first = store.apply_presence(enter(&owner, 1)).await.unwrap();
  router
    .project_presence(committed(&first).change(), store.as_ref())
    .await
    .unwrap();
  assert!(rx.try_recv().is_err());
  let attached = ProtocolMessage::heartbeat();
  assert!(
    !router
      .finish_attach("room", &owner.connection_id, &attached, &initial)
      .await
      .unwrap()
  );
  let snapshot = store.snapshot(room()).await.unwrap();
  assert!(
    router
      .finish_attach("room", &owner.connection_id, &attached, &snapshot)
      .await
      .unwrap()
  );
  rx.try_recv().unwrap();
  rx.try_recv().unwrap();
  router
    .project_presence(committed(&first).change(), store.as_ref())
    .await
    .unwrap();
  assert!(rx.try_recv().is_err());
  let second = store.apply_presence(enter(&owner, 2)).await.unwrap();
  let mut leave = enter(&owner, 3);
  leave.items[0].action = PresenceMutationAction::Leave;
  let third = store.apply_presence(leave).await.unwrap();
  router
    .project_presence(committed(&third).change(), store.as_ref())
    .await
    .unwrap();
  let frame = rx.try_recv().unwrap().into_websocket_message();
  let axum::extract::ws::Message::Text(text) = frame else {
    panic!()
  };
  let sync: ProtocolMessage = serde_json::from_str(&text).unwrap();
  assert!(matches!(sync.action, ProtocolAction::Sync));
  assert!(sync.presence.unwrap().is_empty());
  router
    .project_presence(committed(&second).change(), store.as_ref())
    .await
    .unwrap();
  assert!(rx.try_recv().is_err());
}

#[tokio::test]
#[ignore = "requires an isolated Redis"]
async fn lost_node_lease_stops_runtime_and_new_mutations() {
  let f = Fixture::new().await;
  let instance = NodeInstance::new(
    NodeId::try_new("lost").unwrap(),
    BootGeneration::generate(),
    Timestamp::now(),
  );
  let runtime = RedisPresenceRuntime::claim(
    f.redis.clone(),
    f.keys.clone(),
    &instance,
    PresenceLedgerPolicy {
      capacity: 3,
      retention: Duration::from_secs(1),
    },
  )
  .await
  .unwrap();
  let store = runtime.store();
  f.expire(&store).await;
  let bus = Arc::new(EventBus::with_distributed_publisher(
    Arc::new(EventDispatcher::new()),
    Arc::new(RecordingPublisher::default()),
  ));
  assert!(
    tokio::time::timeout(Duration::from_secs(2), runtime.run(bus))
      .await
      .unwrap()
      .is_err()
  );
  assert!(!store.is_ready());
  assert!(
    store
      .apply_presence(enter(&actor(&store), 1))
      .await
      .is_err()
  );
}

#[tokio::test]
#[ignore = "requires an isolated Redis"]
async fn projection_failure_is_retryable_and_disables_mutations() {
  let f = Fixture::new().await;
  let store = f.node("failed-projection").await;
  let owner = actor(&store);
  attach(&store, &owner).await;
  store.apply_presence(enter(&owner, 1)).await.unwrap();
  let receipt = store.apply_presence(enter(&owner, 2)).await.unwrap();
  let realtime = Arc::new(Realtime::from_redis(
    RealtimeConfig {
      application_id: owner.application_id.clone(),
      key_name: "test.key".into(),
      key_secret: "test-secret".into(),
    },
    store.clone(),
  ));
  let application = realtime.application(&owner.application_id).unwrap();
  let (shutdown, _listener) = shutdown_channel();
  let (tx, _rx) = tokio::sync::mpsc::channel(16);
  application
    .router()
    .attach(
      "room",
      owner.connection_id.clone(),
      OutboundSender::new(tx, shutdown),
    )
    .await;
  // A revision gap requires a snapshot; corrupt only this fixture's state to fail the read.
  f.redis
    .invoke_script(
      "redis.call('DEL', KEYS[1]); return redis.call('SET', KEYS[1], 'wrong-type')",
      &[f.keys.channel_state(&room()).as_bytes()],
      &[],
    )
    .await
    .unwrap();
  let mut dispatcher = EventDispatcher::new();
  crate::register_event_handlers(&mut dispatcher, realtime.clone()).unwrap();
  let error = dispatcher
    .dispatch(&EventMessage::try_from(committed(&receipt)).unwrap())
    .await
    .unwrap_err();
  assert!(error.is_retryable());
  assert!(!realtime.is_ready());
  assert!(store.apply_presence(enter(&owner, 3)).await.is_err());
}

#[tokio::test]
#[ignore = "requires an isolated Redis"]
async fn dropping_runtime_disables_store_before_lease_expiry() {
  let f = Fixture::new().await;
  let instance = NodeInstance::new(
    NodeId::try_new("cancelled").unwrap(),
    BootGeneration::generate(),
    Timestamp::now(),
  );
  let runtime = RedisPresenceRuntime::claim(
    f.redis.clone(),
    f.keys.clone(),
    &instance,
    PresenceLedgerPolicy::from_settings(&ApplicationSettings::default()),
  )
  .await
  .unwrap();
  let store = runtime.store();
  assert!(store.is_ready());
  drop(runtime);
  assert!(!store.is_ready());
  assert!(
    store
      .apply_presence(enter(&actor(&store), 1))
      .await
      .is_err()
  );
}
