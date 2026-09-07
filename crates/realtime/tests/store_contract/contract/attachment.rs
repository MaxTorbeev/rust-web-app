//! Attach, detach и disconnect: занятость канала, идемпотентность, server-side
//! leave, атомарность многоканального отключения.

use realtime::{
  AttachmentTracking, ChannelMode, ChannelStateStoreError, CommittedChannelTransition,
  OccupancyCategory, PresenceChangeAction,
};
use serde_json::json;

use pretty_assertions::assert_eq;
use rstest_reuse::apply;

use super::ContractStore;
use super::fixtures::*;
use crate::stores;

/// Первый attach — событие с occupancy (без presence-ревизии) и кандидатом
/// `event_id`; snapshot уже учитывает соединение.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn first_attach_reports_occupancy_with_candidate_event_id(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");

  let command = attach_cmd(&conn, &room, &ALL_MODES, at(0));
  let candidate = command.event_id;
  let outcome = store
    .attach_and_snapshot(command)
    .await
    .expect("attach must succeed");

  let transition = &outcome.transition;
  assert_eq!(event_id(transition), candidate);
  let change = event(transition).change();
  assert_eq!(
    change.presence_revision, None,
    "attach does not change the member list"
  );
  assert!(change.member_changes.is_empty());
  assert_eq!(change.channel, room);
  assert_eq!(change.occurred_at, at(0));

  let occupancy = change
    .occupancy
    .as_ref()
    .expect("first attach must change occupancy");
  assert_eq!(occupancy.metrics.connections, 1);
  assert_eq!(occupancy.metrics.presence_connections, 1);
  assert!(
    occupancy
      .zero_boundary_categories
      .contains(&OccupancyCategory::Connections)
  );
  assert_eq!(
    occupancy.metrics, outcome.snapshot.occupancy,
    "event and snapshot must agree"
  );
  assert_eq!(change.occupancy_version, outcome.snapshot.occupancy_version);
  assert_eq!(outcome.snapshot.presence_revision, 0);
}

/// Повторный attach того же соединения ничего не считает дважды и возвращает
/// свежий snapshot, а не тот, что был при первом attach.
///
/// Ловит: удвоение `connections`, `Changed` без изменений, устаревший snapshot.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn reattach_is_idempotent_and_returns_fresh_snapshot(#[case] store: impl ContractStore) {
  let store = &store;
  let (conn, other) = (Conn::new("c1"), Conn::new("c2"));
  let room = channel("room");
  let first = attach(store, &conn, &room, at(0)).await;

  // Между двумя attach канал изменился: вошёл участник другого соединения.
  attach(store, &other, &room, at(1)).await;
  apply(
    store,
    presence_cmd(&other, &room, 1, vec![enter("bob", json!(null))], at(2)),
  )
  .await;
  let before = snapshot(store, &room).await;

  let second = attach(store, &conn, &room, at(3)).await;

  assert!(
    matches!(second.transition, CommittedChannelTransition::Unchanged { occupancy_version } if occupancy_version == before.occupancy_version),
    "{:?}",
    second.transition
  );
  assert_eq!(
    second.snapshot.occupancy.connections, 2,
    "re-attach must not double count"
  );
  assert_eq!(
    client_ids(&second.snapshot),
    vec!["bob"],
    "snapshot must be fresh, not the one from the first attach"
  );
  assert_ne!(
    first.snapshot.presence_revision,
    second.snapshot.presence_revision
  );
  assert_unchanged(&before, &snapshot(store, &room).await);
}

/// Повторный attach с другими режимами заменяет attachment: mode-based метрики
/// пересчитываются и это — изменение occupancy.
///
/// Ловит: игнорирование нового attachment при повторе.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn reattach_with_different_modes_updates_mode_counters(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");
  store
    .attach_and_snapshot(attach_cmd(&conn, &room, &[ChannelMode::Subscribe], at(0)))
    .await
    .expect("attach must succeed");
  let before = snapshot(store, &room).await;
  assert_eq!(
    (before.occupancy.subscribers, before.occupancy.publishers),
    (1, 0)
  );

  let outcome = store
    .attach_and_snapshot(attach_cmd(
      &conn,
      &room,
      &[ChannelMode::Subscribe, ChannelMode::Publish],
      at(1),
    ))
    .await
    .expect("attach must succeed");

  let change = event(&outcome.transition).change();
  let occupancy = change
    .occupancy
    .as_ref()
    .expect("mode change must be reported");
  assert_eq!(occupancy.metrics.publishers, 1);
  assert_eq!(occupancy.metrics.connections, 1, "still one connection");
  assert!(
    occupancy
      .changed_categories
      .contains(&OccupancyCategory::Publishers)
  );
  assert!(
    !occupancy
      .changed_categories
      .contains(&OccupancyCategory::Connections)
  );
  assert!(outcome.snapshot.occupancy_version > before.occupancy_version);
}

/// Агрегированный учёт и канал другого приложения отклоняются до изменения
/// состояния.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn attach_rejects_aggregated_accounting_and_foreign_channels(
  #[case] store: impl ContractStore,
) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");

  let mut aggregated = attach_cmd(&conn, &room, &ALL_MODES, at(0));
  aggregated.accounting = AttachmentTracking::Aggregated;
  let result = store.attach_and_snapshot(aggregated).await;
  assert!(
    matches!(result, Err(ChannelStateStoreError::InvalidRequest { .. })),
    "{result:?}"
  );

  let foreign = realtime::ChannelKey::new(realtime::ApplicationId::new(OTHER_APP), "room");
  let result = store
    .attach_and_snapshot(attach_cmd(&conn, &foreign, &ALL_MODES, at(1)))
    .await;
  assert!(
    matches!(result, Err(ChannelStateStoreError::InvalidRequest { .. })),
    "{result:?}"
  );

  let untouched = snapshot(store, &room).await;
  assert_eq!(untouched.occupancy.connections, 0);
  assert_eq!(untouched.occupancy_version, 0);
}

/// Detach соединения с участниками: одно событие, `Leave` на каждого в порядке
/// `client_id`, server-generated `message_id`, ревизия +1, occupancy без этого
/// соединения.
///
/// Ловит: событие на участника, недетерминированный порядок, client-style
/// `message_id`, чужой `event_id`.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn detach_with_members_emits_sorted_server_leaves(#[case] store: impl ContractStore) {
  let store = &store;
  let (conn, other) = (Conn::new("c1"), Conn::new("c2"));
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;
  attach(store, &other, &room, at(0)).await;
  apply(
    store,
    presence_cmd(
      &conn,
      &room,
      1,
      vec![
        enter("zoe", json!("z")),
        enter("adam", json!("a")),
        enter("mia", json!("m")),
      ],
      at(1),
    ),
  )
  .await;
  apply(
    store,
    presence_cmd(&other, &room, 1, vec![enter("bob", json!(null))], at(2)),
  )
  .await;
  let before = snapshot(store, &room).await;

  let command = detach_cmd(&conn, &room, at(3));
  let candidate = command.event_id;
  let transition = store.detach(command).await.expect("detach must succeed");

  assert_eq!(event_id(&transition), candidate);
  let change = event(&transition).change();
  assert_eq!(
    change.presence_revision,
    Some(before.presence_revision + 1),
    "one revision for the whole detach"
  );

  let leaves: Vec<_> = change
    .member_changes
    .iter()
    .map(|delta| (delta.action, delta.client_id.as_str()))
    .collect();
  assert_eq!(
    leaves,
    vec![
      (PresenceChangeAction::Leave, "adam"),
      (PresenceChangeAction::Leave, "mia"),
      (PresenceChangeAction::Leave, "zoe"),
    ]
  );
  for (index, delta) in change.member_changes.iter().enumerate() {
    assert_eq!(delta.message_id, format!("server:{candidate}:{index}"));
    assert_eq!(delta.connection_id.as_str(), conn.id());
    assert_eq!(delta.timestamp, at(3));
  }
  assert_eq!(
    change.member_changes[0].data,
    Some(json!("a")),
    "leave carries the member's data"
  );

  let after = snapshot(store, &room).await;
  assert_eq!(
    client_ids(&after),
    vec!["bob"],
    "other connection's member must survive"
  );
  assert_eq!(after.occupancy.connections, 1);
  assert_eq!(after.occupancy.presence_members, 1);
  let occupancy = change
    .occupancy
    .as_ref()
    .expect("detach must report occupancy");
  assert_eq!(occupancy.metrics, after.occupancy);
}

/// Detach без участников меняет только occupancy: ревизия Presence не растёт.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn detach_without_members_changes_only_occupancy(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;
  let before = snapshot(store, &room).await;

  let transition = store
    .detach(detach_cmd(&conn, &room, at(1)))
    .await
    .expect("detach must succeed");
  let change = event(&transition).change();

  assert_eq!(change.presence_revision, None);
  assert!(change.member_changes.is_empty());
  assert_eq!(
    change
      .occupancy
      .as_ref()
      .map(|occupancy| occupancy.metrics.connections),
    Some(0)
  );

  let after = snapshot(store, &room).await;
  assert_eq!(after.presence_revision, before.presence_revision);
  assert!(after.occupancy_version > before.occupancy_version);
}

/// Detach неприсоединённого соединения — успешный no-op с текущей версией
/// occupancy, и для канала, которого никогда не было, тоже.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn detach_of_unknown_attachment_is_unchanged(#[case] store: impl ContractStore) {
  let store = &store;
  let (conn, other) = (Conn::new("c1"), Conn::new("c2"));
  let room = channel("room");
  attach(store, &other, &room, at(0)).await;
  let before = snapshot(store, &room).await;

  let transition = store
    .detach(detach_cmd(&conn, &room, at(1)))
    .await
    .expect("detach must succeed");
  assert!(
    matches!(transition, CommittedChannelTransition::Unchanged { occupancy_version } if occupancy_version == before.occupancy_version),
    "{transition:?}"
  );
  assert_unchanged(&before, &snapshot(store, &room).await);

  let transition = store
    .detach(detach_cmd(&conn, &channel("never"), at(2)))
    .await
    .expect("detach must succeed");
  assert!(
    matches!(
      transition,
      CommittedChannelTransition::Unchanged {
        occupancy_version: 0
      }
    ),
    "{transition:?}"
  );
  assert!(snapshot(store, &channel("never")).await.members.is_empty());
}

/// Disconnect покрывает все каналы соединения: по событию на канал в
/// детерминированном порядке, `event_id` выведен из кандидата, участники
/// удалены везде, повтор — пустой список.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn disconnect_covers_all_channels_in_deterministic_order(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let (a, b, c) = (channel("a-room"), channel("b-room"), channel("c-room"));
  // Attach в неалфавитном порядке: порядок событий не должен от него зависеть.
  for room in [&c, &a, &b] {
    attach(store, &conn, room, at(0)).await;
    apply(
      store,
      presence_cmd(&conn, room, 1, vec![enter("alice", json!(null))], at(1)),
    )
    .await;
  }

  let command = disconnect_cmd(&conn, at(2));
  let expected_ids = [&a, &b, &c].map(|room| command.channel_event_id(room));
  let transitions = store
    .disconnect(command)
    .await
    .expect("disconnect must succeed");

  assert_eq!(transitions.len(), 3);
  let channels: Vec<_> = transitions
    .iter()
    .map(|t| event(t).change().channel.channel.clone())
    .collect();
  assert_eq!(channels, vec!["a-room", "b-room", "c-room"]);
  let ids: Vec<_> = transitions.iter().map(event_id).collect();
  assert_eq!(
    ids,
    expected_ids.to_vec(),
    "channel event ids must derive from the command candidate"
  );

  for room in [&a, &b, &c] {
    let after = snapshot(store, room).await;
    assert!(after.members.is_empty());
    assert_eq!(after.occupancy.connections, 0);
  }

  let again = store
    .disconnect(disconnect_cmd(&conn, at(3)))
    .await
    .expect("repeated disconnect must succeed");
  assert!(again.is_empty());
}

/// Disconnect атомарен: конфликт владения в одном канале оставляет нетронутыми
/// все остальные.
///
/// Ловит: detach каналов по одному с остановкой на ошибке.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn disconnect_is_all_or_nothing_on_ownership_conflict(#[case] store: impl ContractStore) {
  let store = &store;
  let owner = Conn::on_node("c1", "node-a");
  let intruder = Conn::on_node("c1", "node-b");
  let (mine, theirs) = (channel("mine"), channel("theirs"));
  attach(store, &owner, &mine, at(0)).await;
  apply(
    store,
    presence_cmd(&owner, &mine, 1, vec![enter("alice", json!(null))], at(1)),
  )
  .await;
  // Тот же connection_id, но attachment принадлежит другому экземпляру ноды.
  attach(store, &intruder, &theirs, at(2)).await;
  let mine_before = snapshot(store, &mine).await;
  let theirs_before = snapshot(store, &theirs).await;

  let result = store.disconnect(disconnect_cmd(&owner, at(3))).await;

  assert!(
    matches!(result, Err(ChannelStateStoreError::Conflict { .. })),
    "{result:?}"
  );
  assert_unchanged(&mine_before, &snapshot(store, &mine).await);
  assert_unchanged(&theirs_before, &snapshot(store, &theirs).await);
  assert_eq!(
    client_ids(&snapshot(store, &mine).await),
    vec!["alice"],
    "own channel must not be detached"
  );
}

/// Mode-based метрики считаются из effective modes attachment-а.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn occupancy_counters_follow_effective_modes(#[case] store: impl ContractStore) {
  let store = &store;
  let (viewer, publisher) = (Conn::new("viewer"), Conn::new("publisher"));
  let room = channel("room");

  store
    .attach_and_snapshot(attach_cmd(&viewer, &room, &[ChannelMode::Subscribe], at(0)))
    .await
    .expect("attach must succeed");
  let outcome = store
    .attach_and_snapshot(attach_cmd(
      &publisher,
      &room,
      &[ChannelMode::Publish, ChannelMode::Presence],
      at(1),
    ))
    .await
    .expect("attach must succeed");

  let metrics = outcome.snapshot.occupancy;
  assert_eq!(metrics.connections, 2);
  assert_eq!(metrics.subscribers, 1);
  assert_eq!(metrics.publishers, 1);
  assert_eq!(metrics.presence_connections, 1);
  assert_eq!(metrics.presence_subscribers, 0);
  assert_eq!(
    metrics.presence_members, 0,
    "presence mode alone does not make a member"
  );
}

/// Симметричная последовательность возвращает все счётчики в ноль, а
/// `occupancy_version` при этом строго растёт на каждом `Changed`.
///
/// Ловит: дрейф счётчиков, повтор или откат версии.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn counters_return_to_zero_and_versions_never_repeat(#[case] store: impl ContractStore) {
  let store = &store;
  let room = channel("room");
  let conns: Vec<Conn> = (1..=3).map(|i| Conn::new(&format!("c{i}"))).collect();
  let mut versions = Vec::new();

  for (i, conn) in conns.iter().enumerate() {
    let outcome = attach(store, conn, &room, at(i as u64)).await;
    versions.push(outcome.transition.occupancy_version());
    let receipt = apply(
      store,
      presence_cmd(
        conn,
        &room,
        1,
        vec![enter(&format!("m{i}"), json!(null))],
        at(10 + i as u64),
      ),
    )
    .await;
    versions.push(committed(&receipt).occupancy_version());
  }
  assert_eq!(snapshot(store, &room).await.occupancy.presence_members, 3);

  let mid = store
    .detach(detach_cmd(&conns[0], &room, at(20)))
    .await
    .expect("detach must succeed");
  versions.push(mid.occupancy_version());
  for conn in &conns[1..] {
    let transitions = store
      .disconnect(disconnect_cmd(conn, at(21)))
      .await
      .expect("disconnect must succeed");
    versions.extend(
      transitions
        .iter()
        .map(CommittedChannelTransition::occupancy_version),
    );
  }

  let after = snapshot(store, &room).await;
  assert_eq!(
    (
      after.occupancy.connections,
      after.occupancy.presence_members,
      after.occupancy.publishers,
      after.occupancy.subscribers
    ),
    (0, 0, 0, 0)
  );
  assert!(
    versions.windows(2).all(|pair| pair[0] < pair[1]),
    "occupancy versions must strictly increase: {versions:?}"
  );
  assert_eq!(*versions.last().unwrap(), after.occupancy_version);
}
