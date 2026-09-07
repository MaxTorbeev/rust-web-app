//! Snapshot: чтение без побочных эффектов и согласованность с событиями.

use serde_json::json;

use pretty_assertions::assert_eq;
use rstest_reuse::apply;

use super::ContractStore;
use super::fixtures::*;
use crate::stores;

/// Неизвестный канал — пустой snapshot с нулевыми версиями и метриками, и его
/// чтение не создаёт канал.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn unknown_channel_snapshot_is_empty_and_side_effect_free(#[case] store: impl ContractStore) {
  let store = &store;
  let room = channel("nobody-here");

  let first = snapshot(store, &room).await;
  assert!(first.members.is_empty());
  assert_eq!((first.presence_revision, first.occupancy_version), (0, 0));
  assert_eq!(first.occupancy.connections, 0);
  assert_eq!(first.occupancy.presence_members, 0);

  // Первый attach после чтения всё ещё видит канал как новый.
  let conn = Conn::new("c1");
  let outcome = attach(store, &conn, &room, at(0)).await;
  assert_eq!(outcome.snapshot.presence_revision, 0);
}

/// Snapshot не меняет состояние: подряд идущие чтения идентичны, версии не растут.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn snapshot_is_read_only(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;
  apply(
    store,
    presence_cmd(
      &conn,
      &room,
      1,
      vec![enter("alice", json!({"k": "v"}))],
      at(1),
    ),
  )
  .await;

  let first = snapshot(store, &room).await;
  let second = snapshot(store, &room).await;
  assert_unchanged(&first, &second);
  assert_eq!(first.members[0].data, second.members[0].data);
}

/// Snapshot содержит участников всех соединений с полными полями, а его
/// occupancy равен метрикам последнего события.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn snapshot_lists_members_of_all_connections(#[case] store: impl ContractStore) {
  let store = &store;
  let (c1, c2) = (Conn::on_node("c1", "node-a"), Conn::on_node("c2", "node-a"));
  let room = channel("room");
  attach(store, &c1, &room, at(0)).await;
  attach(store, &c2, &room, at(0)).await;
  apply(
    store,
    presence_cmd(&c1, &room, 1, vec![enter("alice", json!(1))], at(1)),
  )
  .await;
  let last = apply(
    store,
    presence_cmd(&c2, &room, 1, vec![enter("bob", json!(2))], at(2)),
  )
  .await;

  let snapshot = snapshot(store, &room).await;
  assert_eq!(client_ids(&snapshot), vec!["alice", "bob"]);

  let bob = snapshot
    .members
    .iter()
    .find(|m| m.client_id == "bob")
    .expect("bob must be present");
  assert_eq!(bob.connection_id.as_str(), c2.id());
  assert_eq!(bob.node_instance, c2.node);
  assert_eq!(bob.data, Some(json!(2)));
  assert_eq!(bob.presence_revision, snapshot.presence_revision);
  assert_eq!(bob.last_message_id, format!("{}:1:0", c2.id()));

  let change = event(committed(&last)).change();
  assert_eq!(
    change.occupancy.as_ref().map(|o| o.metrics.clone()),
    Some(snapshot.occupancy)
  );
  assert_eq!(change.occupancy_version, snapshot.occupancy_version);
}
