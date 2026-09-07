//! Мутации Presence: enter/update/leave, batch, политика идентификации,
//! предусловия.
//!
//! Сценарии проверяют не «команда прошла», а форму зафиксированного события и
//! состояние после него: ровно одна ревизия на batch, порядок дельт, стабильные
//! `message_id`, occupancy только при изменении числа участников, атомарность
//! отказа.

use realtime::{
  ChannelMode, ChannelStateStoreError, CommittedChannelTransition, OccupancyCategory,
  PresenceChangeAction, PresenceClientIdPolicy, PresenceMutationAction, PresenceRejection,
};
use serde_json::json;

use pretty_assertions::assert_eq;
use rstest_reuse::apply;

use super::ContractStore;
use super::fixtures::*;
use crate::stores;

/// ENTER: одна дельта `Enter`, ревизия +1, `message_id` в формате
/// `connectionId:msgSerial:index`, occupancy отражает нового участника и
/// совпадает со snapshot.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn enter_commits_one_delta_with_stable_message_id(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");
  let attached = attach(store, &conn, &room, at(0)).await;
  let revision_before = attached.snapshot.presence_revision;
  let occupancy_version_before = attached.snapshot.occupancy_version;

  let command = presence_cmd(
    &conn,
    &room,
    5,
    vec![enter("alice", json!({"status": "online"}))],
    at(1),
  );
  let candidate = command.event_id;
  let receipt = apply(store, command).await;
  let transition = committed(&receipt);
  let change = event(transition).change();

  assert_eq!(
    event_id(transition),
    candidate,
    "store must use the candidate event_id"
  );
  assert_eq!(change.presence_revision, Some(revision_before + 1));
  assert_eq!(change.member_changes.len(), 1);

  let delta = &change.member_changes[0];
  assert_eq!(delta.action, PresenceChangeAction::Enter);
  assert_eq!(delta.client_id, "alice");
  assert_eq!(delta.connection_id.as_str(), conn.id());
  assert_eq!(delta.message_id, format!("{}:5:0", conn.id()));
  assert_eq!(delta.data, Some(json!({"status": "online"})));
  assert_eq!(delta.timestamp, at(1));

  let after = snapshot(store, &room).await;
  assert_eq!(after.presence_revision, revision_before + 1);
  assert!(
    after.occupancy_version > occupancy_version_before,
    "new member must change occupancy"
  );
  assert_eq!(change.occupancy_version, after.occupancy_version);
  assert_eq!(after.occupancy.presence_members, 1);

  let occupancy = change
    .occupancy
    .as_ref()
    .expect("enter must carry an occupancy change");
  assert_eq!(
    occupancy.metrics, after.occupancy,
    "event occupancy must equal the snapshot after commit"
  );
  assert!(
    occupancy
      .changed_categories
      .contains(&OccupancyCategory::PresenceMembers)
  );
  assert!(
    occupancy
      .zero_boundary_categories
      .contains(&OccupancyCategory::PresenceMembers)
  );

  let member = &after.members[0];
  assert_eq!(member.client_id, "alice");
  assert_eq!(member.last_message_id, delta.message_id);
  assert_eq!(member.presence_revision, after.presence_revision);
  assert_eq!(member.updated_at_ms, at(1).as_millis());
}

/// UPDATE меняет данные и ревизию, но не число участников: occupancy не меняется
/// ни в событии, ни в версии.
///
/// Ловит: продвижение `occupancy_version` на каждую мутацию, `occupancy: Some`
/// без изменения метрик, потерю данных.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn update_advances_revision_but_not_occupancy(#[case] store: impl ContractStore) {
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
      vec![enter("alice", json!({"v": 1}))],
      at(1),
    ),
  )
  .await;
  let before = snapshot(store, &room).await;

  let receipt = apply(
    store,
    presence_cmd(
      &conn,
      &room,
      2,
      vec![update("alice", json!({"v": 2}))],
      at(2),
    ),
  )
  .await;
  let change = event(committed(&receipt)).change();

  assert_eq!(
    change.member_changes[0].action,
    PresenceChangeAction::Update
  );
  assert_eq!(change.member_changes[0].data, Some(json!({"v": 2})));
  assert_eq!(change.presence_revision, Some(before.presence_revision + 1));
  assert!(
    change.occupancy.is_none(),
    "update must not report an occupancy change"
  );
  assert_eq!(change.occupancy_version, before.occupancy_version);

  let after = snapshot(store, &room).await;
  assert_eq!(after.occupancy_version, before.occupancy_version);
  assert_eq!(after.occupancy, before.occupancy);
  assert_eq!(after.members.len(), 1);
  assert_eq!(after.members[0].data, Some(json!({"v": 2})));
  assert_eq!(
    after.members[0].last_message_id,
    format!("{}:2:0", conn.id())
  );
}

/// Повторный ENTER присутствующего участника — это UPDATE: участник не
/// дублируется, событие несёт `Update`.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn re_enter_is_an_update(#[case] store: impl ContractStore) {
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
      vec![enter("alice", json!({"v": 1}))],
      at(1),
    ),
  )
  .await;

  let receipt = apply(
    store,
    presence_cmd(
      &conn,
      &room,
      2,
      vec![enter("alice", json!({"v": 2}))],
      at(2),
    ),
  )
  .await;
  let change = event(committed(&receipt)).change();

  assert_eq!(
    change.member_changes[0].action,
    PresenceChangeAction::Update
  );
  assert!(change.occupancy.is_none());

  let after = snapshot(store, &room).await;
  assert_eq!(client_ids(&after), vec!["alice"]);
  assert_eq!(after.members[0].data, Some(json!({"v": 2})));
}

/// LEAVE удаляет участника, уменьшает occupancy и публикует данные: из
/// сообщения, если они есть, иначе последние данные участника.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn leave_removes_member_and_reports_last_data(#[case] store: impl ContractStore) {
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
      vec![
        enter("alice", json!({"last": true})),
        enter("bob", json!(null)),
      ],
      at(1),
    ),
  )
  .await;
  let before = snapshot(store, &room).await;

  let receipt = apply(
    store,
    presence_cmd(&conn, &room, 2, vec![leave("alice")], at(2)),
  )
  .await;
  let change = event(committed(&receipt)).change();
  assert_eq!(change.member_changes[0].action, PresenceChangeAction::Leave);
  assert_eq!(
    change.member_changes[0].data,
    Some(json!({"last": true})),
    "leave without data must carry the member's last data"
  );

  let receipt = apply(
    store,
    presence_cmd(
      &conn,
      &room,
      3,
      vec![leave_with("bob", json!({"bye": 1}))],
      at(3),
    ),
  )
  .await;
  let change = event(committed(&receipt)).change();
  assert_eq!(
    change.member_changes[0].data,
    Some(json!({"bye": 1})),
    "leave data from the message wins"
  );

  let after = snapshot(store, &room).await;
  assert!(after.members.is_empty());
  assert_eq!(after.occupancy.presence_members, 0);
  assert_eq!(after.presence_revision, before.presence_revision + 2);
  assert!(after.occupancy_version > before.occupancy_version);
  let occupancy = change
    .occupancy
    .as_ref()
    .expect("last leave must report occupancy");
  assert!(
    occupancy
      .zero_boundary_categories
      .contains(&OccupancyCategory::PresenceMembers)
  );
}

/// UPDATE и LEAVE отсутствующего участника — доменный отказ без изменений.
/// Каждое действие — отдельный тест (`::update`, `::leave`), чтобы падение
/// указывало на конкретное.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn update_or_leave_of_absent_member_is_rejected(
  #[case] store: impl ContractStore,
  #[values(PresenceMutationAction::Update, PresenceMutationAction::Leave)]
  action: PresenceMutationAction,
) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;
  apply(
    store,
    presence_cmd(&conn, &room, 1, vec![enter("alice", json!(null))], at(1)),
  )
  .await;
  let before = snapshot(store, &room).await;

  let receipt = apply(
    store,
    presence_cmd(
      &conn,
      &room,
      2,
      vec![item(action, Some("ghost"), None)],
      at(2),
    ),
  )
  .await;

  assert_eq!(rejected(&receipt), &PresenceRejection::InvalidMemberState);
  assert_unchanged(&before, &snapshot(store, &room).await);
}

/// Batch атомарен: отказ на любом элементе откатывает все предыдущие.
///
/// Ловит: частичное применение (первые элементы вошли, потом отказ), сдвиг
/// ревизии при отказе.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn rejected_batch_applies_nothing(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;
  let before = snapshot(store, &room).await;

  let receipt = apply(
    store,
    presence_cmd(
      &conn,
      &room,
      1,
      vec![
        enter("alice", json!(null)),
        enter("bob", json!(null)),
        update("ghost", json!(null)),
      ],
      at(1),
    ),
  )
  .await;

  assert_eq!(rejected(&receipt), &PresenceRejection::InvalidMemberState);
  let after = snapshot(store, &room).await;
  assert_unchanged(&before, &after);
  assert!(
    after.members.is_empty(),
    "no element of a rejected batch may be applied"
  );
}

/// Элементы batch применяются последовательно к рабочей копии: LEAVE может
/// удалить участника, вошедшего в том же batch. Одна ревизия, одно событие,
/// дельты в порядке элементов с индексами в `message_id`.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn mixed_batch_is_one_revision_in_element_order(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;
  let before = snapshot(store, &room).await;

  let receipt = apply(
    store,
    presence_cmd(
      &conn,
      &room,
      9,
      vec![
        enter("alice", json!(1)),
        enter("bob", json!(2)),
        leave("alice"),
      ],
      at(1),
    ),
  )
  .await;
  let change = event(committed(&receipt)).change();

  let actions: Vec<_> = change
    .member_changes
    .iter()
    .map(|delta| (delta.action, delta.client_id.as_str()))
    .collect();
  assert_eq!(
    actions,
    vec![
      (PresenceChangeAction::Enter, "alice"),
      (PresenceChangeAction::Enter, "bob"),
      (PresenceChangeAction::Leave, "alice"),
    ]
  );
  let ids: Vec<_> = change
    .member_changes
    .iter()
    .map(|delta| delta.message_id.clone())
    .collect();
  assert_eq!(
    ids,
    vec![
      format!("{}:9:0", conn.id()),
      format!("{}:9:1", conn.id()),
      format!("{}:9:2", conn.id())
    ]
  );
  assert_eq!(change.presence_revision, Some(before.presence_revision + 1));
  assert_eq!(
    change.member_changes[2].data,
    Some(json!(1)),
    "leave in the same batch reports the entered data"
  );

  let after = snapshot(store, &room).await;
  assert_eq!(client_ids(&after), vec!["bob"]);
  assert_eq!(
    after.presence_revision,
    before.presence_revision + 1,
    "a batch is exactly one revision"
  );
  assert_eq!(after.occupancy.presence_members, 1);
}

/// Несколько `client_id` одного соединения независимы.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn multiple_client_ids_per_connection_are_independent(#[case] store: impl ContractStore) {
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
      vec![enter("alice", json!(1)), enter("bob", json!(2))],
      at(1),
    ),
  )
  .await;
  apply(
    store,
    presence_cmd(&conn, &room, 2, vec![leave("alice")], at(2)),
  )
  .await;

  let after = snapshot(store, &room).await;
  assert_eq!(client_ids(&after), vec!["bob"]);
  assert_eq!(after.members[0].data, Some(json!(2)));
  assert_eq!(after.occupancy.presence_members, 1);
  assert_eq!(
    after.occupancy.presence_connections, 1,
    "presence connections count attachments, not members"
  );
}

/// Участники разных соединений не смешиваются: leave одного соединения не трогает
/// одноимённого участника другого.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn members_are_scoped_by_connection(#[case] store: impl ContractStore) {
  let store = &store;
  let (c1, c2) = (Conn::new("c1"), Conn::new("c2"));
  let room = channel("room");
  attach(store, &c1, &room, at(0)).await;
  attach(store, &c2, &room, at(0)).await;
  apply(
    store,
    presence_cmd(&c1, &room, 1, vec![enter("alice", json!("c1"))], at(1)),
  )
  .await;
  apply(
    store,
    presence_cmd(&c2, &room, 1, vec![enter("alice", json!("c2"))], at(2)),
  )
  .await;

  assert_eq!(snapshot(store, &room).await.occupancy.presence_members, 2);

  apply(
    store,
    presence_cmd(&c1, &room, 2, vec![leave("alice")], at(3)),
  )
  .await;

  let after = snapshot(store, &room).await;
  assert_eq!(after.members.len(), 1);
  assert_eq!(after.members[0].connection_id.as_str(), c2.id());
  assert_eq!(after.members[0].data, Some(json!("c2")));
}

/// Политика идентификации: неидентифицированное соединение не может заявить
/// никакой `client_id`, `Bound` отклоняет чужие идентификаторы (с указанием
/// какого), `Any` пропускает.
///
/// `UnidentifiedConnection` зарезервирован для элемента без `client_id` вовсе
/// (см. `missing_client_id_is_rejected`); попытка неидентифицированного
/// соединения назваться кем-то — это `ClientIdNotAllowed`.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn client_id_policy_is_enforced(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;

  let unidentified = apply(
    store,
    presence_cmd_with_policy(
      &conn,
      &room,
      1,
      vec![enter("alice", json!(null))],
      at(1),
      PresenceClientIdPolicy::Unidentified,
    ),
  )
  .await;
  assert_eq!(
    rejected(&unidentified),
    &PresenceRejection::ClientIdNotAllowed {
      client_id: "alice".to_owned()
    },
  );

  let foreign = apply(
    store,
    presence_cmd_with_policy(
      &conn,
      &room,
      2,
      vec![enter("alice", json!(null)), enter("mallory", json!(null))],
      at(2),
      bound(&["alice"]),
    ),
  )
  .await;
  assert_eq!(
    rejected(&foreign),
    &PresenceRejection::ClientIdNotAllowed {
      client_id: "mallory".to_owned()
    },
  );
  assert!(
    snapshot(store, &room).await.members.is_empty(),
    "alice from the rejected batch must not be entered"
  );

  let own = apply(
    store,
    presence_cmd_with_policy(
      &conn,
      &room,
      3,
      vec![enter("alice", json!(null))],
      at(3),
      bound(&["alice"]),
    ),
  )
  .await;
  committed(&own);

  let any = apply(
    store,
    presence_cmd_with_policy(
      &conn,
      &room,
      4,
      vec![enter("anyone", json!(null))],
      at(4),
      PresenceClientIdPolicy::Any,
    ),
  )
  .await;
  committed(&any);
}

/// Элемент без `client_id` (ни в сообщении, ни у соединения) — отказ
/// `UnidentifiedConnection` даже при разрешающей политике.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn missing_client_id_is_rejected(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;

  let receipt = apply(
    store,
    presence_cmd(
      &conn,
      &room,
      1,
      vec![item(PresenceMutationAction::Enter, None, Some(json!(null)))],
      at(1),
    ),
  )
  .await;
  assert_eq!(
    rejected(&receipt),
    &PresenceRejection::UnidentifiedConnection
  );
}

/// Предусловия: без attach — `NotAttached`; attachment без режима `Presence` —
/// `PresenceModeNotEnabled`; в обоих случаях состояние не меняется.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn presence_requires_attachment_with_presence_mode(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");

  let receipt = apply(
    store,
    presence_cmd(&conn, &room, 1, vec![enter("alice", json!(null))], at(0)),
  )
  .await;
  assert_eq!(rejected(&receipt), &PresenceRejection::NotAttached);

  store
    .attach_and_snapshot(attach_cmd(
      &conn,
      &room,
      &[ChannelMode::Subscribe, ChannelMode::PresenceSubscribe],
      at(1),
    ))
    .await
    .expect("attach must succeed");
  let before = snapshot(store, &room).await;

  let receipt = apply(
    store,
    presence_cmd(&conn, &room, 2, vec![enter("alice", json!(null))], at(2)),
  )
  .await;
  assert_eq!(
    rejected(&receipt),
    &PresenceRejection::PresenceModeNotEnabled
  );
  assert_unchanged(&before, &snapshot(store, &room).await);
}

/// Attachment другого экземпляра ноды — нарушение владения: ошибка хранилища,
/// а не доменный отказ клиенту.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn foreign_node_attachment_is_a_store_conflict(#[case] store: impl ContractStore) {
  let store = &store;
  let owner = Conn::on_node("c1", "node-a");
  let intruder = Conn::on_node("c1", "node-b");
  let room = channel("room");
  attach(store, &owner, &room, at(0)).await;

  let result = store
    .apply_presence(presence_cmd(
      &intruder,
      &room,
      1,
      vec![enter("alice", json!(null))],
      at(1),
    ))
    .await;

  assert!(
    matches!(result, Err(ChannelStateStoreError::Conflict { .. })),
    "{result:?}"
  );
  assert!(snapshot(store, &room).await.members.is_empty());
}

/// Канал другого приложения в команде — ошибка запроса до любого поиска в
/// журнале и без записи в него.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn cross_application_channel_is_an_invalid_request(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let foreign_room = realtime::ChannelKey::new(realtime::ApplicationId::new(OTHER_APP), "room");

  let result = store
    .apply_presence(presence_cmd(
      &conn,
      &foreign_room,
      1,
      vec![enter("alice", json!(null))],
      at(0),
    ))
    .await;
  assert!(
    matches!(result, Err(ChannelStateStoreError::InvalidRequest { .. })),
    "{result:?}"
  );

  // Тот же serial в своём канале — новая операция, а не конфликт: ошибка не записана.
  let room = channel("room");
  attach(store, &conn, &room, at(1)).await;
  let receipt = apply(
    store,
    presence_cmd(&conn, &room, 1, vec![enter("alice", json!(null))], at(2)),
  )
  .await;
  assert!(!receipt.replayed);
  committed(&receipt);
}

/// Каждое presence-событие несёт ровно ту ревизию, которую показывает snapshot
/// после него, и ревизии строго возрастают на 1.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn revisions_are_strictly_sequential(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;

  let mut previous = snapshot(store, &room).await.presence_revision;
  let items = [
    enter("alice", json!(1)),
    update("alice", json!(2)),
    enter("bob", json!(3)),
    leave("alice"),
    leave("bob"),
  ];

  for (index, item) in items.into_iter().enumerate() {
    let serial = index as u64 + 1;
    let receipt = apply(
      store,
      presence_cmd(&conn, &room, serial, vec![item], at(serial)),
    )
    .await;
    let transition = committed(&receipt);
    let CommittedChannelTransition::Changed(event) = transition else {
      panic!("every presence mutation must produce an event");
    };

    let revision = event
      .change()
      .presence_revision
      .expect("presence event must carry a revision");
    assert_eq!(
      revision,
      previous + 1,
      "revision must advance by exactly one"
    );
    assert_eq!(snapshot(store, &room).await.presence_revision, revision);
    previous = revision;
  }
}
