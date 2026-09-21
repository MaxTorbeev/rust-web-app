//! Схема хранения Redis v1: кодирование сегментов ключей.
//!
//! Сериализация значений и ответы Lua будут добавляться вместе с transitions.
//! Версия относится к сохранённым данным, даже если Rust-модули приватны.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use redis_lease::{LeaseOwner, RedisLeaseError};
use support::NodeInstance;

pub(super) const SUBSYSTEM: &str = "presence";
pub(super) const SCHEMA_VERSION: u64 = 1;

/// UTF-8 без нормализации. Алфавит не содержит разделителей `.` и `:`.
pub(super) fn segment(value: &str) -> String {
  URL_SAFE_NO_PAD.encode(value.as_bytes())
}

/// Ключ операции в HASH/ZSET: строки одинаковой длины сортируются как u64.
pub(super) fn operation_serial(value: u64) -> String {
  format!("{value:020}")
}

/// Формирует строковый идентификатор запуска ноды из node_id и boot_generation.
/// Используется в ключах и индексах Redis, а также как владелец node lease.
/// started_at не участвует: это метаданные запуска.
pub(super) fn generation(instance: &NodeInstance) -> String {
  format!("{}.{}", segment(instance.node_id.as_str()), instance.boot_generation.as_uuid())
}

/// Формирование идентификатора запуска приложения из node_id и boot_generation.
pub(super) fn node_lease_owner(instance: &NodeInstance) -> Result<LeaseOwner, RedisLeaseError> {
  LeaseOwner::new(generation(instance))
}
