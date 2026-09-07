//! Конструкторы реализаций для шаблона `stores`.

use std::time::Duration;

use realtime::{MemoryChannelStore, PresenceLedgerPolicy};

use super::contract::fixtures::{LEDGER_CAPACITY, LEDGER_RETENTION_MS};

/// `MemoryChannelStore` с ограничениями журнала, на которые рассчитаны сценарии.
pub fn memory_store() -> MemoryChannelStore {
  MemoryChannelStore::with_ledger_policy(PresenceLedgerPolicy {
    capacity: LEDGER_CAPACITY,
    retention: Duration::from_millis(LEDGER_RETENTION_MS),
  })
}

/// Property-тесты не ложатся в шаблон `stores` (proptest синхронен и сам
/// управляет циклом), поэтому вызываются на реализацию явно.
#[test]
fn memory_ledger_matches_reference_model() {
  crate::contract::ledger_model::check_against_model(memory_store);
}
