//! Contract test suite хранилища состояния каналов.
//!
//! Сценарии в `contract/` написаны против трейтов `AttachmentStore` и
//! `PresenceStore` и не знают о реализации. Шаблон `stores` перечисляет
//! реализации; каждый сценарий применяет его через `#[apply(stores)]` и тем
//! самым становится тестом для каждой из них (`<сценарий>::case_1_memory`).
//! Redis-реализация подключается одной строкой `#[case::redis(...)]` здесь.

use rstest_reuse::{self, template};

mod contract;
mod implementations;

// Шаблон раскрывается в модуле сценария, поэтому все пути в нём абсолютные.
#[template]
#[rstest::rstest]
#[case::memory(crate::implementations::memory_store())]
fn stores(#[case] store: impl crate::contract::ContractStore) {}
