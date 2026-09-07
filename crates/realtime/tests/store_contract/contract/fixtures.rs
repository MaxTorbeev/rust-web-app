//! Конструкторы команд и проверки, общие для всех сценариев.
//!
//! Время всегда передаётся явно: ни один сценарий не зависит от системных
//! часов, retention проверяется сдвигом `Timestamp`, а не ожиданием.

use std::collections::BTreeSet;

use pretty_assertions::assert_eq;

use realtime::{
  ApplicationId, AttachCommand, AttachmentTracking, ChannelAttachOutcome, ChannelKey, ChannelMode,
  CommittedChannelTransition, CommittedPresenceEvent, ConnectionActor, ConnectionId, DetachCommand,
  DisconnectConnectionCommand, PresenceActor, PresenceBatchCommand, PresenceBatchItem,
  PresenceClientIdPolicy, PresenceMutationAction, PresenceMutationOutcome, PresenceMutationReceipt,
  PresenceRejection, PresenceSnapshot,
};
use serde_json::{Value, json};
use support::{BootGeneration, NodeId, NodeInstance, fresh_uuid, timestamp::Timestamp};
use uuid::Uuid;

use super::ContractStore;

pub const APP: &str = "application-1";
pub const OTHER_APP: &str = "application-2";
pub const T0: u64 = 1_700_000_000_000;

pub fn at(offset_ms: u64) -> Timestamp {
  Timestamp::from_millis(T0 + offset_ms)
}

pub fn node(name: &str) -> NodeInstance {
  NodeInstance::new(
    NodeId::try_new(name).expect("test node id must be valid"),
    BootGeneration::generate(),
    at(0),
  )
}

pub fn channel(name: &str) -> ChannelKey {
  ChannelKey::new(ApplicationId::new(APP), name)
}

/// Соединение с фиксированным идентификатором: сценарии повторяют один и тот же
/// `connection_id` на разных нодах и в разных приложениях намеренно.
#[derive(Clone)]
pub struct Conn {
  pub application_id: ApplicationId,
  pub connection_id: ConnectionId,
  pub node: NodeInstance,
}

impl Conn {
  pub fn new(connection_id: &str) -> Self {
    Self::on_node(connection_id, "node-a")
  }

  pub fn on_node(connection_id: &str, node_name: &str) -> Self {
    Self {
      application_id: ApplicationId::new(APP),
      connection_id: ConnectionId::from_test_str(connection_id),
      node: node(node_name),
    }
  }

  pub fn in_application(mut self, application_id: &str) -> Self {
    self.application_id = ApplicationId::new(application_id);
    self
  }

  pub fn actor(&self) -> ConnectionActor {
    ConnectionActor {
      application_id: self.application_id.clone(),
      connection_id: self.connection_id.clone(),
      node_instance: self.node.clone(),
    }
  }

  pub fn id(&self) -> &str {
    self.connection_id.as_str()
  }
}

/// `ConnectionId` из произвольной строки для тестов: у типа есть только
/// генератор, а сценариям нужны предсказуемые идентификаторы.
pub trait TestConnectionId {
  fn from_test_str(value: &str) -> ConnectionId;
}

impl TestConnectionId for ConnectionId {
  fn from_test_str(value: &str) -> ConnectionId {
    serde_json::from_value(json!(value)).expect("connection id must deserialize from a string")
  }
}

pub const ALL_MODES: [ChannelMode; 4] = [
  ChannelMode::Subscribe,
  ChannelMode::Publish,
  ChannelMode::Presence,
  ChannelMode::PresenceSubscribe,
];

pub fn attach_cmd(
  conn: &Conn,
  channel: &ChannelKey,
  modes: &[ChannelMode],
  time: Timestamp,
) -> AttachCommand {
  AttachCommand {
    channel: channel.clone(),
    actor: conn.actor(),
    accounting: AttachmentTracking::Individual,
    effective_modes: modes.to_vec(),
    occupancy: None,
    request_time: time,
    event_id: fresh_uuid(),
  }
}

pub fn detach_cmd(conn: &Conn, channel: &ChannelKey, time: Timestamp) -> DetachCommand {
  DetachCommand {
    channel: channel.clone(),
    actor: conn.actor(),
    request_time: time,
    event_id: fresh_uuid(),
  }
}

pub fn disconnect_cmd(conn: &Conn, time: Timestamp) -> DisconnectConnectionCommand {
  DisconnectConnectionCommand {
    actor: conn.actor(),
    request_time: time,
    event_id: fresh_uuid(),
  }
}

pub fn enter(client_id: &str, data: Value) -> PresenceBatchItem {
  item(PresenceMutationAction::Enter, Some(client_id), Some(data))
}

pub fn update(client_id: &str, data: Value) -> PresenceBatchItem {
  item(PresenceMutationAction::Update, Some(client_id), Some(data))
}

pub fn leave(client_id: &str) -> PresenceBatchItem {
  item(PresenceMutationAction::Leave, Some(client_id), None)
}

pub fn leave_with(client_id: &str, data: Value) -> PresenceBatchItem {
  item(PresenceMutationAction::Leave, Some(client_id), Some(data))
}

pub fn item(
  action: PresenceMutationAction,
  client_id: Option<&str>,
  data: Option<Value>,
) -> PresenceBatchItem {
  PresenceBatchItem {
    action,
    client_id: client_id.map(str::to_owned),
    data,
  }
}

/// Fingerprint, детерминированный по содержимому batch: одинаковые элементы дают
/// одинаковую строку, любое отличие — другую. Store сравнивает его как непрозрачную
/// строку, поэтому конкретная функция хеширования контракту безразлична.
pub fn fingerprint(channel: &ChannelKey, items: &[PresenceBatchItem]) -> String {
  let normalized: Vec<Value> = items
    .iter()
    .map(|item| json!({ "action": item.action.as_str(), "clientId": item.client_id, "data": item.data }))
    .collect();

  json!({ "channel": channel.channel, "items": normalized }).to_string()
}

pub fn presence_cmd(
  conn: &Conn,
  channel: &ChannelKey,
  msg_serial: u64,
  items: Vec<PresenceBatchItem>,
  time: Timestamp,
) -> PresenceBatchCommand {
  presence_cmd_with_policy(
    conn,
    channel,
    msg_serial,
    items,
    time,
    PresenceClientIdPolicy::Any,
  )
}

pub fn presence_cmd_with_policy(
  conn: &Conn,
  channel: &ChannelKey,
  msg_serial: u64,
  items: Vec<PresenceBatchItem>,
  time: Timestamp,
  client_id_policy: PresenceClientIdPolicy,
) -> PresenceBatchCommand {
  PresenceBatchCommand {
    channel: channel.clone(),
    actor: PresenceActor {
      connection_actor: conn.actor(),
      client_id_policy,
    },
    request_fingerprint: fingerprint(channel, &items),
    items,
    msg_serial,
    request_time: time,
    event_id: fresh_uuid(),
  }
}

pub fn bound(client_ids: &[&str]) -> PresenceClientIdPolicy {
  PresenceClientIdPolicy::Bound(
    client_ids
      .iter()
      .map(|id| (*id).to_owned())
      .collect::<BTreeSet<_>>(),
  )
}

// ---------------------------------------------------------------------------
// Shorthand for the most common store calls.
// ---------------------------------------------------------------------------

pub async fn attach(
  store: &impl ContractStore,
  conn: &Conn,
  channel: &ChannelKey,
  time: Timestamp,
) -> ChannelAttachOutcome {
  store
    .attach_and_snapshot(attach_cmd(conn, channel, &ALL_MODES, time))
    .await
    .expect("attach must succeed")
}

pub async fn apply(
  store: &impl ContractStore,
  command: PresenceBatchCommand,
) -> PresenceMutationReceipt {
  store
    .apply_presence(command)
    .await
    .expect("presence command must not fail with a store error")
}

pub async fn snapshot(store: &impl ContractStore, channel: &ChannelKey) -> PresenceSnapshot {
  store
    .snapshot(channel.clone())
    .await
    .expect("snapshot must succeed")
}

// ---------------------------------------------------------------------------
// Assertions that read the outcome shape and fail with a useful message.
// ---------------------------------------------------------------------------

pub fn committed(receipt: &PresenceMutationReceipt) -> &CommittedChannelTransition {
  match &receipt.outcome {
    PresenceMutationOutcome::Committed(transition) => transition,
    PresenceMutationOutcome::Rejected(rejection) => {
      panic!("expected a committed outcome, got rejection {rejection:?}")
    }
  }
}

pub fn rejected(receipt: &PresenceMutationReceipt) -> &PresenceRejection {
  match &receipt.outcome {
    PresenceMutationOutcome::Rejected(rejection) => rejection,
    PresenceMutationOutcome::Committed(transition) => {
      panic!("expected a rejection, got committed transition {transition:?}")
    }
  }
}

pub fn event(transition: &CommittedChannelTransition) -> &CommittedPresenceEvent {
  transition
    .event()
    .unwrap_or_else(|| panic!("expected a transition with an event, got {transition:?}"))
}

pub fn event_id(transition: &CommittedChannelTransition) -> Uuid {
  event(transition).event_id()
}

pub fn client_ids(snapshot: &PresenceSnapshot) -> Vec<String> {
  let mut ids: Vec<String> = snapshot
    .members
    .iter()
    .map(|member| member.client_id.clone())
    .collect();
  ids.sort();
  ids
}

/// Всё, что должно быть неизменным после отказа или повтора: участники, обе
/// версии и метрики. Сравнивается по значению, а не по ссылке на snapshot.
pub fn assert_unchanged(before: &PresenceSnapshot, after: &PresenceSnapshot) {
  assert_eq!(client_ids(before), client_ids(after), "member set changed");
  assert_eq!(
    before.presence_revision, after.presence_revision,
    "presence revision changed"
  );
  assert_eq!(
    before.occupancy_version, after.occupancy_version,
    "occupancy version changed"
  );
  assert_eq!(
    before.occupancy, after.occupancy,
    "occupancy metrics changed"
  );
}

/// Ограничения журнала, с которыми раннер обязан собрать хранилище: сценарии
/// ledger рассчитаны именно на эти значения.
pub const LEDGER_CAPACITY: usize = 3;
pub const LEDGER_RETENTION_MS: u64 = 1_000;
