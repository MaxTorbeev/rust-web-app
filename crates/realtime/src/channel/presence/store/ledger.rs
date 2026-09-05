use super::PresenceOperationRecord;
use crate::{ApplicationSettings, PresenceMutationOutcome};
use std::collections::BTreeMap;
use std::time::Duration;
use support::timestamp::Timestamp;

/// Ограничения журнала Presence-операций.
///
/// `capacity` ограничивает память открытого журнала: клиент не может раздуть
/// её произвольным числом уникальных `msg_serial`. `retention` ограничивает
/// время жизни закрытого журнала и должен быть не меньше окна retry/resume
/// соединения.
#[derive(Clone, Copy, Debug)]
pub struct PresenceLedgerPolicy {
  /// Максимальное число записей одного соединения; старшие `msg_serial`
  /// вытесняют младшие.
  pub capacity: usize,

  /// Время хранения закрытого журнала после disconnect.
  pub retention: Duration,
}

impl PresenceLedgerPolicy {
  /// Выводит политику из настроек приложения: retention равен окну
  /// восстановления соединения `connection_state_ttl`.
  pub fn from_settings(settings: &ApplicationSettings) -> Self {
    Self {
      capacity: usize::try_from(settings.presence_ledger_capacity)
        .unwrap_or(usize::MAX)
        .max(1),
      retention: Duration::from_millis(settings.connection_state_ttl),
    }
  }
}

impl Default for PresenceLedgerPolicy {
  fn default() -> Self {
    Self::from_settings(&ApplicationSettings::default())
  }
}

/// Журнал Presence-операций одного соединения.
///
/// Ключ записи — `msg_serial`; журнал сам привязан к
/// `(application_id, connection_id)`. Тип не зависит от хранилища: memory и
/// Redis реализации применяют одни и те же правила поиска и записи.
///
/// Журнал хранит окно из последних `capacity` операций и наибольший виденный
/// `msg_serial`. Операция, вытесненная из окна, не может быть воспроизведена,
/// но и новой операцией стать не может: её повтор отклоняется.
///
/// Жизненный цикл: журнал открывается первой операцией соединения, закрывается
/// авторитетным disconnect и удаляется очисткой не раньше, чем через retention
/// после закрытия. Закрытый журнал доступен только для чтения: повтор
/// известной операции воспроизводится, неизвестная операция отклоняется.
#[derive(Debug)]
pub(crate) struct PresenceOperationLedger {
  records: BTreeMap<u64, PresenceOperationRecord>,
  highest_serial: Option<u64>,
  capacity: usize,
  state: LedgerState,
}

#[derive(Clone, Copy, Debug, Default)]
enum LedgerState {
  #[default]
  Open,

  /// Соединение завершено; журнал ожидает очистки.
  Closed { closed_at: Timestamp },
}

/// Результат поиска операции в журнале.
#[derive(Debug)]
pub(crate) enum LedgerLookup<'a> {
  /// Операция ещё не выполнялась, журнал открыт: команду можно применить.
  Miss,

  /// Повтор уже выполненной операции: вернуть прежний результат без изменений.
  Replay(&'a PresenceMutationOutcome),

  /// Тот же `msg_serial` с другим содержимым запроса.
  Conflict,

  /// `msg_serial` старше окна хранения: операция уже вытеснена, её результат
  /// восстановить нельзя, новой операцией она стать не может.
  Evicted,

  /// Журнал закрыт, операция неизвестна: новой операцией стать не может.
  Closed,
}

impl PresenceOperationLedger {
  pub(crate) fn new(capacity: usize) -> Self {
    Self {
      records: BTreeMap::new(),
      highest_serial: None,
      capacity: capacity.max(1),
      state: LedgerState::Open,
    }
  }

  pub(crate) fn lookup(&self, msg_serial: u64, request_fingerprint: &str) -> LedgerLookup<'_> {
    // Известная операция воспроизводится независимо от состояния журнала:
    // поздний повтор после disconnect должен получить прежний ACK или NACK.
    if let Some(record) = self.records.get(&msg_serial) {
      return if record.matches(request_fingerprint) {
        LedgerLookup::Replay(record.outcome())
      } else {
        LedgerLookup::Conflict
      };
    }

    if self.is_evicted(msg_serial) {
      return LedgerLookup::Evicted;
    }

    match self.state {
      LedgerState::Open => LedgerLookup::Miss,
      LedgerState::Closed { .. } => LedgerLookup::Closed,
    }
  }

  /// `msg_serial` не старше наибольшего виденного, но младше всех сохранённых
  /// записей: он мог быть вытеснен из окна. Пропуски внутри окна вытеснением
  /// не считаются.
  fn is_evicted(&self, msg_serial: u64) -> bool {
    let Some(highest) = self.highest_serial else {
      return false;
    };

    if msg_serial > highest {
      return false;
    }

    match self.records.keys().next() {
      Some(&lowest_kept) => msg_serial < lowest_kept,
      // Записей нет, но операции были: всё окно вытеснено.
      None => true,
    }
  }

  /// Сохраняет результат впервые выполненной операции.
  ///
  /// Вызывается только после `LedgerLookup::Miss` в той же критической секции,
  /// что и сама операция. При переполнении вытесняются записи с наименьшими
  /// `msg_serial`. Повторная запись того же `msg_serial` или запись в закрытый
  /// журнал нарушает инвариант и в debug-сборке завершается паникой.
  pub(crate) fn record(&mut self, msg_serial: u64, record: PresenceOperationRecord) {
    debug_assert!(self.is_open(), "presence ledger is closed");

    let previous = self.records.insert(msg_serial, record);

    debug_assert!(
      previous.is_none(),
      "presence operation {msg_serial} is already recorded"
    );

    self.highest_serial = Some(
      self
        .highest_serial
        .map_or(msg_serial, |highest| highest.max(msg_serial)),
    );

    while self.records.len() > self.capacity {
      self.records.pop_first();
    }
  }

  /// Закрывает журнал при авторитетном завершении соединения.
  ///
  /// Повторное закрытие не сдвигает `closed_at`, иначе retention можно было бы
  /// продлевать бесконечно повторными disconnect.
  pub(crate) fn close(&mut self, closed_at: Timestamp) {
    if self.is_open() {
      self.state = LedgerState::Closed { closed_at };
    }
  }

  pub(crate) const fn is_open(&self) -> bool {
    matches!(self.state, LedgerState::Open)
  }

  pub(crate) const fn is_closed(&self) -> bool {
    !self.is_open()
  }

  /// Возвращает `true`, если журнал закрыт и retention после закрытия истёк.
  ///
  /// Retention должен быть не меньше максимального окна retry/resume: после
  /// удаления журнала поздний повтор уже не отличим от новой операции.
  pub(crate) fn is_expired(&self, now: Timestamp, retention: Duration) -> bool {
    match self.state {
      LedgerState::Open => false,
      LedgerState::Closed { closed_at } => {
        let retention_ms = u64::try_from(retention.as_millis()).unwrap_or(u64::MAX);
        let expires_at = closed_at.as_millis().saturating_add(retention_ms);

        now.as_millis() >= expires_at
      }
    }
  }
}
