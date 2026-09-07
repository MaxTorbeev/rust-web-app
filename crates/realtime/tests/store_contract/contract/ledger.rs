//! Дедупликация клиентских Presence-команд.
//!
//! Каждый сценарий целится в конкретный способ сломать журнал: выполнить повтор
//! заново, затереть запись конфликтом, записать инфраструктурную ошибку,
//! перепутать соединения или приложения, потерять окно вытеснения, пережить
//! закрытие журнала или продлить retention повторным disconnect.

use realtime::{ChannelMode, ChannelStateStoreError, PresenceRejection};
use serde_json::json;

use pretty_assertions::assert_eq;
use rstest_reuse::apply;

use super::ContractStore;
use super::fixtures::*;
use crate::stores;

/// Повтор committed-команды с другим кандидатом `event_id` возвращает событие с
/// исходным идентификатором и ничего не меняет.
///
/// Ловит: повторное выполнение вместо воспроизведения (второй Enter, вторая
/// ревизия), использование нового кандидата при replay, `replayed = false`.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn replay_returns_original_event_and_leaves_state_untouched(
  #[case] store: impl ContractStore,
) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;

  let first = presence_cmd(
    &conn,
    &room,
    1,
    vec![enter("alice", json!({"v": 1}))],
    at(10),
  );
  let mut retry = first.clone();
  retry.event_id = support::fresh_uuid();
  retry.request_time = at(500);

  let original = apply(store, first).await;
  assert!(!original.replayed);
  let before = snapshot(store, &room).await;

  let replayed = apply(store, retry).await;

  assert!(
    replayed.replayed,
    "second delivery of the same msg_serial must be reported as replayed"
  );
  assert_eq!(
    event_id(committed(&replayed)),
    event_id(committed(&original))
  );
  assert_eq!(
    committed(&replayed).presence_revision(),
    committed(&original).presence_revision(),
  );
  assert_unchanged(&before, &snapshot(store, &room).await);
  assert_eq!(
    client_ids(&snapshot(store, &room).await),
    vec!["alice"],
    "member must not be duplicated"
  );
}

/// Отказ воспроизводится даже когда его причина исчезла.
///
/// Ловит: журнал хранит только committed-исходы, или store сначала проверяет
/// предусловия и лишь потом смотрит в журнал.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn rejected_outcome_is_replayed_after_precondition_changes(
  #[case] store: impl ContractStore,
) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");

  // ENTER до attach — доменный отказ, который должен попасть в журнал.
  let command = presence_cmd(&conn, &room, 1, vec![enter("alice", json!(null))], at(0));
  assert_eq!(
    rejected(&apply(store, command.clone()).await),
    &PresenceRejection::NotAttached
  );

  // Теперь attach выполнен, и та же команда прошла бы — но это повтор.
  attach(store, &conn, &room, at(1)).await;
  let replayed = apply(store, command).await;

  assert!(replayed.replayed);
  assert_eq!(rejected(&replayed), &PresenceRejection::NotAttached);
  assert!(
    snapshot(store, &room).await.members.is_empty(),
    "replayed rejection must not enter the member"
  );
}

/// Повтор известной операции после detach всё ещё возвращает исходный результат.
///
/// Ловит: проверку attachment раньше поиска в журнале.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn replay_wins_over_current_attachment_state(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;

  let command = presence_cmd(&conn, &room, 1, vec![enter("alice", json!(null))], at(1));
  let original = apply(store, command.clone()).await;

  store
    .detach(detach_cmd(&conn, &room, at(2)))
    .await
    .expect("detach must succeed");

  let replayed = apply(store, command).await;
  assert!(replayed.replayed);
  assert_eq!(
    event_id(committed(&replayed)),
    event_id(committed(&original))
  );
  assert!(
    snapshot(store, &room).await.members.is_empty(),
    "replay must not re-enter after detach"
  );
}

/// Тот же `msg_serial` с другим содержимым — protocol conflict, и он не затирает
/// исходную запись.
///
/// Ловит: перезапись записи журнала последним запросом, выполнение конфликтной
/// команды как новой, `replayed = true` у конфликта.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn conflicting_payload_is_rejected_without_overwriting_the_record(
  #[case] store: impl ContractStore,
) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;

  let original_cmd = presence_cmd(
    &conn,
    &room,
    7,
    vec![enter("alice", json!({"v": 1}))],
    at(1),
  );
  let original = apply(store, original_cmd.clone()).await;
  let before = snapshot(store, &room).await;

  let conflicting = presence_cmd(
    &conn,
    &room,
    7,
    vec![enter("mallory", json!({"v": 2}))],
    at(2),
  );
  let conflict = apply(store, conflicting).await;

  assert_eq!(rejected(&conflict), &PresenceRejection::ConflictingReplay);
  assert!(
    !conflict.replayed,
    "conflict is a fresh verdict, not a replay"
  );
  assert_unchanged(&before, &snapshot(store, &room).await);

  // Исходная команда по-прежнему воспроизводится.
  let replayed = apply(store, original_cmd).await;
  assert!(replayed.replayed);
  assert_eq!(
    event_id(committed(&replayed)),
    event_id(committed(&original))
  );
}

/// Инфраструктурная ошибка не записывается: следующая попытка с тем же
/// `msg_serial` выполняется как первая.
///
/// Ловит: запись `Err` в журнал (тогда вторая команда с другим fingerprint
/// получила бы `ConflictingReplay`).
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn store_error_is_not_recorded(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;

  // Пустой batch — ошибка запроса, а не доменный отказ.
  let error = store
    .apply_presence(presence_cmd(&conn, &room, 3, vec![], at(1)))
    .await
    .expect_err("empty batch must be a store error");
  assert!(
    matches!(error, ChannelStateStoreError::InvalidRequest { .. }),
    "{error:?}"
  );

  let retry = apply(
    store,
    presence_cmd(&conn, &room, 3, vec![enter("alice", json!(null))], at(2)),
  )
  .await;
  assert!(!retry.replayed);
  committed(&retry);
  assert_eq!(client_ids(&snapshot(store, &room).await), vec!["alice"]);
}

/// Журналы разных соединений независимы: одинаковые `msg_serial` не мешают
/// друг другу.
///
/// Ловит: ключ журнала без `connection_id`.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn ledgers_are_isolated_per_connection(#[case] store: impl ContractStore) {
  let store = &store;
  let (c1, c2) = (Conn::new("c1"), Conn::new("c2"));
  let room = channel("room");
  attach(store, &c1, &room, at(0)).await;
  attach(store, &c2, &room, at(0)).await;

  let first = apply(
    store,
    presence_cmd(&c1, &room, 1, vec![enter("alice", json!(null))], at(1)),
  )
  .await;
  let second = apply(
    store,
    presence_cmd(&c2, &room, 1, vec![enter("bob", json!(null))], at(2)),
  )
  .await;

  assert!(
    !second.replayed,
    "another connection's serial must not be treated as a replay"
  );
  assert_ne!(event_id(committed(&first)), event_id(committed(&second)));
  assert_eq!(
    client_ids(&snapshot(store, &room).await),
    vec!["alice", "bob"]
  );
}

/// Одинаковый `connection_id` в разных приложениях — разные журналы.
///
/// Ловит: ключ журнала без `application_id`.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn ledgers_are_isolated_per_application(#[case] store: impl ContractStore) {
  let store = &store;
  let conn_a = Conn::new("shared-id");
  let conn_b = Conn::new("shared-id").in_application(OTHER_APP);
  let room_a = channel("room");
  let room_b = realtime::ChannelKey::new(realtime::ApplicationId::new(OTHER_APP), "room");
  attach(store, &conn_a, &room_a, at(0)).await;
  attach(store, &conn_b, &room_b, at(0)).await;

  apply(
    store,
    presence_cmd(
      &conn_a,
      &room_a,
      1,
      vec![enter("alice", json!(null))],
      at(1),
    ),
  )
  .await;
  let second = apply(
    store,
    presence_cmd(&conn_b, &room_b, 1, vec![enter("bob", json!(null))], at(2)),
  )
  .await;

  assert!(!second.replayed);
  committed(&second);
  assert_eq!(client_ids(&snapshot(store, &room_b).await), vec!["bob"]);
}

/// Окно хранения: вытесненный `msg_serial` отклоняется как устаревший, а не
/// становится новой операцией; записи внутри окна воспроизводятся.
///
/// Ловит: отсутствие границы размера, вытеснение не самых старых записей,
/// выполнение вытесненного serial заново.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn evicted_serial_is_rejected_as_stale(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;

  let capacity = LEDGER_CAPACITY as u64;
  let mut commands = Vec::new();

  // capacity + 1 операций: самая старая должна быть вытеснена.
  for serial in 1..=capacity + 1 {
    let command = presence_cmd(
      &conn,
      &room,
      serial,
      vec![enter(&format!("m{serial}"), json!(null))],
      at(serial),
    );
    commands.push(command.clone());
    committed(&apply(store, command).await);
  }
  let before = snapshot(store, &room).await;

  let stale = apply(store, commands[0].clone()).await;
  assert_eq!(rejected(&stale), &PresenceRejection::StaleOperation);
  assert!(!stale.replayed);
  assert_unchanged(&before, &snapshot(store, &room).await);

  // Самая старая из оставшихся и самая новая — воспроизводятся.
  assert!(apply(store, commands[1].clone()).await.replayed);
  assert!(
    apply(store, commands[capacity as usize].clone())
      .await
      .replayed
  );
}

/// Пропуск внутри окна — не вытеснение: такой serial выполняется как новая
/// операция. Serial больше любого виденного — тоже.
///
/// Ловит: слишком грубое правило «serial ≤ highest ⇒ stale».
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn gap_inside_window_is_a_new_operation(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;

  // Serial 1 и 10: окно [1, 10], внутри пропуск.
  committed(
    &apply(
      store,
      presence_cmd(&conn, &room, 1, vec![enter("a", json!(null))], at(1)),
    )
    .await,
  );
  committed(
    &apply(
      store,
      presence_cmd(&conn, &room, 10, vec![enter("b", json!(null))], at(2)),
    )
    .await,
  );

  let gap = apply(
    store,
    presence_cmd(&conn, &room, 5, vec![enter("c", json!(null))], at(3)),
  )
  .await;
  assert!(!gap.replayed);
  committed(&gap);

  let ahead = apply(
    store,
    presence_cmd(&conn, &room, 11, vec![enter("d", json!(null))], at(4)),
  )
  .await;
  assert!(!ahead.replayed);
  committed(&ahead);

  assert_eq!(
    client_ids(&snapshot(store, &room).await),
    vec!["a", "b", "c", "d"]
  );
}

/// После полного вытеснения окна (записей нет, но операции были) любой старый
/// serial устаревший.
///
/// Ловит: «нет записей ⇒ журнал пустой ⇒ Miss».
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn fully_evicted_window_still_rejects_old_serials(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;

  let capacity = LEDGER_CAPACITY as u64;
  // Первая партия заполняет окно, вторая полностью вытесняет первую.
  for serial in 1..=2 * capacity {
    committed(
      &apply(
        store,
        presence_cmd(
          &conn,
          &room,
          serial,
          vec![enter(&format!("m{serial}"), json!(null))],
          at(serial),
        ),
      )
      .await,
    );
  }

  let stale = apply(
    store,
    presence_cmd(&conn, &room, 1, vec![enter("m1", json!(null))], at(100)),
  )
  .await;
  assert_eq!(rejected(&stale), &PresenceRejection::StaleOperation);
}

/// Закрытый журнал: известный serial воспроизводится, неизвестный — отказ
/// `ConnectionClosed`, а не новая операция; attach тем же соединением — конфликт.
///
/// Ловит: удаление журнала при disconnect, выполнение поздней команды после
/// disconnect, возрождение завершённого connection_id.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn closed_ledger_replays_known_and_rejects_unknown(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;

  let command = presence_cmd(&conn, &room, 1, vec![enter("alice", json!(null))], at(1));
  let original = apply(store, command.clone()).await;

  store
    .disconnect(disconnect_cmd(&conn, at(2)))
    .await
    .expect("disconnect must succeed");
  let before = snapshot(store, &room).await;
  assert!(
    before.members.is_empty(),
    "disconnect must remove the member"
  );

  let late_replay = apply(store, command).await;
  assert!(late_replay.replayed);
  assert_eq!(
    event_id(committed(&late_replay)),
    event_id(committed(&original))
  );

  let late_new = apply(
    store,
    presence_cmd(&conn, &room, 2, vec![enter("alice", json!(null))], at(3)),
  )
  .await;
  assert_eq!(rejected(&late_new), &PresenceRejection::ConnectionClosed);
  assert!(!late_new.replayed);
  assert_unchanged(&before, &snapshot(store, &room).await);

  let reattach = store
    .attach_and_snapshot(attach_cmd(&conn, &room, &ALL_MODES, at(4)))
    .await;
  assert!(
    matches!(reattach, Err(ChannelStateStoreError::Conflict { .. })),
    "a closed connection must not start a new lifecycle: {reattach:?}",
  );
}

/// `ConnectionClosed` сам в журнал не пишется: тот же serial после очистки
/// журнала не считается конфликтом.
///
/// Ловит: запись отказов о состоянии журнала в сам журнал.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn connection_closed_verdict_is_not_recorded(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let other = Conn::new("c2");
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;
  store
    .disconnect(disconnect_cmd(&conn, at(1)))
    .await
    .expect("disconnect must succeed");

  let late = apply(
    store,
    presence_cmd(&conn, &room, 1, vec![enter("alice", json!(null))], at(2)),
  )
  .await;
  assert_eq!(rejected(&late), &PresenceRejection::ConnectionClosed);

  // Retention истёк; чужой disconnect запускает очистку.
  attach(store, &other, &room, at(3)).await;
  store
    .disconnect(disconnect_cmd(&other, at(1 + LEDGER_RETENTION_MS)))
    .await
    .expect("disconnect must succeed");

  // Тот же serial другим payload: будь ConnectionClosed записан — это был бы конфликт.
  attach(store, &conn, &room, at(1 + LEDGER_RETENTION_MS + 1)).await;
  let fresh = apply(
    store,
    presence_cmd(
      &conn,
      &room,
      1,
      vec![enter("bob", json!(null))],
      at(1 + LEDGER_RETENTION_MS + 2),
    ),
  )
  .await;
  assert!(!fresh.replayed);
  committed(&fresh);
}

/// Retention: до истечения журнал жив (attach конфликтует), после — удалён и тот
/// же `connection_id` может начать новый жизненный цикл.
///
/// Ловит: отсутствие очистки, очистку раньше срока, очистку по времени
/// закрытия «примерно».
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn closed_ledger_is_swept_only_after_retention(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  // Каждый sweeper используется один раз: его собственный журнал после
  // disconnect тоже закрыт, и повторный attach был бы конфликтом.
  let (early_sweeper, late_sweeper) = (Conn::new("sweeper-1"), Conn::new("sweeper-2"));
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;
  store
    .disconnect(disconnect_cmd(&conn, at(100)))
    .await
    .expect("disconnect must succeed");

  // Disconnect другого соединения за 1 мс до истечения retention журнал не удаляет.
  attach(store, &early_sweeper, &room, at(101)).await;
  store
    .disconnect(disconnect_cmd(
      &early_sweeper,
      at(100 + LEDGER_RETENTION_MS - 1),
    ))
    .await
    .expect("disconnect must succeed");
  let early = store
    .attach_and_snapshot(attach_cmd(
      &conn,
      &room,
      &ALL_MODES,
      at(100 + LEDGER_RETENTION_MS - 1),
    ))
    .await;
  assert!(
    matches!(early, Err(ChannelStateStoreError::Conflict { .. })),
    "{early:?}"
  );

  // Ровно по истечении retention очистка удаляет журнал.
  attach(store, &late_sweeper, &room, at(100 + LEDGER_RETENTION_MS)).await;
  store
    .disconnect(disconnect_cmd(&late_sweeper, at(100 + LEDGER_RETENTION_MS)))
    .await
    .expect("disconnect must succeed");
  attach(store, &conn, &room, at(100 + LEDGER_RETENTION_MS + 1)).await;

  let fresh = apply(
    store,
    presence_cmd(
      &conn,
      &room,
      1,
      vec![enter("alice", json!(null))],
      at(100 + LEDGER_RETENTION_MS + 2),
    ),
  )
  .await;
  assert!(!fresh.replayed);
  committed(&fresh);
}

/// Повторный disconnect не сдвигает момент закрытия и не продлевает retention.
///
/// Ловит: `close()` перезаписывающий `closed_at`.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn repeated_disconnect_does_not_extend_retention(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let sweeper = Conn::new("sweeper");
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;
  store
    .disconnect(disconnect_cmd(&conn, at(100)))
    .await
    .expect("disconnect must succeed");

  // Повтор почти в конце retention: если бы он продлевал закрытие, журнал дожил бы дальше.
  let repeated = store
    .disconnect(disconnect_cmd(&conn, at(100 + LEDGER_RETENTION_MS - 1)))
    .await
    .expect("repeated disconnect must succeed");
  assert!(
    repeated.is_empty(),
    "repeated disconnect must not produce transitions"
  );

  attach(store, &sweeper, &room, at(100 + LEDGER_RETENTION_MS)).await;
  store
    .disconnect(disconnect_cmd(&sweeper, at(100 + LEDGER_RETENTION_MS)))
    .await
    .expect("disconnect must succeed");

  attach(store, &conn, &room, at(100 + LEDGER_RETENTION_MS + 1)).await;
}

/// Журнал ведётся и без attach: отказ `NotAttached` записывается и
/// воспроизводится; при этом channel без attach не создаёт участников.
///
/// Ловит: запись в журнал только на пути «канал существует».
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn ledger_records_rejections_for_unknown_channels(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("never-attached");

  let command = presence_cmd(&conn, &room, 1, vec![enter("alice", json!(null))], at(0));
  assert_eq!(
    rejected(&apply(store, command.clone()).await),
    &PresenceRejection::NotAttached
  );

  let conflicting = presence_cmd(&conn, &room, 1, vec![enter("bob", json!(null))], at(1));
  assert_eq!(
    rejected(&apply(store, conflicting).await),
    &PresenceRejection::ConflictingReplay
  );

  let replayed = apply(store, command).await;
  assert!(replayed.replayed);
  assert_eq!(rejected(&replayed), &PresenceRejection::NotAttached);
}

/// Режим без `Presence` — отказ, который тоже дедуплицируется.
#[apply(stores)]
#[tokio::test(flavor = "current_thread")]
async fn mode_rejection_is_recorded(#[case] store: impl ContractStore) {
  let store = &store;
  let conn = Conn::new("c1");
  let room = channel("room");
  store
    .attach_and_snapshot(attach_cmd(&conn, &room, &[ChannelMode::Subscribe], at(0)))
    .await
    .expect("attach must succeed");

  let command = presence_cmd(&conn, &room, 1, vec![enter("alice", json!(null))], at(1));
  assert_eq!(
    rejected(&apply(store, command.clone()).await),
    &PresenceRejection::PresenceModeNotEnabled
  );

  // Re-attach с Presence не меняет уже вынесенный вердикт для того же serial.
  attach(store, &conn, &room, at(2)).await;
  let replayed = apply(store, command).await;
  assert!(replayed.replayed);
  assert_eq!(
    rejected(&replayed),
    &PresenceRejection::PresenceModeNotEnabled
  );
}
