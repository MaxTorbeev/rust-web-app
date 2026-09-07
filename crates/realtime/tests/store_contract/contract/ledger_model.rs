//! Property-тест журнала: произвольная последовательность `msg_serial`
//! сравнивается с эталонной моделью окна дедупликации.
//!
//! Ручные сценарии в `ledger.rs` проверяют случаи, которые автор смог
//! придумать; здесь proptest генерирует последовательности повторов, конфликтов,
//! пропусков и вытеснений, а модель — несколько строк без знания реализации —
//! предсказывает вердикт каждой команды и итоговое множество участников.
//! Инвариант: ни одна операция не выполняется дважды и не пропускается.

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;
use realtime::{PresenceMutationOutcome, PresenceRejection};
use serde_json::json;

use super::ContractStore;
use super::fixtures::*;

/// Вердикт, который модель ожидает от хранилища для одной команды.
#[derive(Debug, PartialEq)]
enum Expected {
  Applied,
  Replayed,
  Conflict,
  Stale,
}

/// Эталон: окно из последних `capacity` записей и наибольший виденный serial.
#[derive(Default)]
struct LedgerModel {
  records: BTreeMap<u64, u8>,
  highest: Option<u64>,
}

impl LedgerModel {
  fn apply(&mut self, serial: u64, payload: u8) -> Expected {
    if let Some(recorded) = self.records.get(&serial) {
      return if *recorded == payload {
        Expected::Replayed
      } else {
        Expected::Conflict
      };
    }

    let below_window = self
      .records
      .keys()
      .next()
      .map_or(true, |lowest| serial < *lowest);

    if self.highest.is_some_and(|highest| serial <= highest) && below_window {
      return Expected::Stale;
    }

    self.records.insert(serial, payload);
    self.highest = Some(self.highest.map_or(serial, |highest| highest.max(serial)));

    while self.records.len() > LEDGER_CAPACITY {
      self.records.pop_first();
    }

    Expected::Applied
  }
}

/// Небольшое пространство serial-ов и payload-ов заставляет генератор
/// постоянно попадать в повторы, конфликты и вытеснения.
fn operations() -> impl Strategy<Value = Vec<(u64, u8)>> {
  prop::collection::vec((0..7u64, 0..3u8), 1..32)
}

/// Прогоняет одну последовательность против хранилища и модели.
pub async fn ledger_matches_reference_model(store: &impl ContractStore, operations: &[(u64, u8)]) {
  let conn = Conn::new("c1");
  let room = channel("room");
  attach(store, &conn, &room, at(0)).await;

  let mut model = LedgerModel::default();
  let mut applied = 0u64;
  let mut members = BTreeSet::new();

  for (step, &(serial, payload)) in operations.iter().enumerate() {
    let client = format!("client-{payload}");
    let command = presence_cmd(
      &conn,
      &room,
      serial,
      vec![enter(&client, json!(payload))],
      at(step as u64 + 1),
    );
    let expected = model.apply(serial, payload);

    let receipt = apply(store, command).await;
    let actual = match (&receipt.outcome, receipt.replayed) {
      (PresenceMutationOutcome::Committed(_), false) => Expected::Applied,
      (PresenceMutationOutcome::Committed(_), true) => Expected::Replayed,
      (PresenceMutationOutcome::Rejected(PresenceRejection::ConflictingReplay), false) => {
        Expected::Conflict
      }
      (PresenceMutationOutcome::Rejected(PresenceRejection::StaleOperation), false) => {
        Expected::Stale
      }
      other => {
        panic!("unexpected receipt at step {step} (serial {serial}, payload {payload}): {other:?}")
      }
    };

    assert_eq!(
      actual,
      expected,
      "step {step}: serial {serial}, payload {payload}, history {:?}",
      &operations[..step]
    );

    if expected == Expected::Applied {
      applied += 1;
      members.insert(client);
    }
  }

  let snapshot = snapshot(store, &room).await;
  assert_eq!(
    client_ids(&snapshot),
    members.into_iter().collect::<Vec<_>>(),
    "member set must reflect exactly the applied operations"
  );
  assert_eq!(
    snapshot.presence_revision, applied,
    "every applied operation is exactly one revision, replays and rejections are none"
  );
}

/// Раннер property-теста для одной реализации: proptest синхронен, поэтому
/// каждая последовательность выполняется на собственном current-thread runtime
/// со свежим хранилищем.
pub fn check_against_model<S: ContractStore>(make_store: impl Fn() -> S) {
  let runtime = tokio::runtime::Builder::new_current_thread()
    .build()
    .expect("test runtime must build");

  proptest!(ProptestConfig::with_cases(256), |(operations in operations())| {
    let store = make_store();
    runtime.block_on(ledger_matches_reference_model(&store, &operations));
  });
}
