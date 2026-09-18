//! Всё Lua крейта: операции lease и фрагменты для чужих скриптов.

pub(crate) const ACQUIRE: &str = concat!(
  include_str!("acquire_lease.lua"),
  "\n",
  include_str!("acquire.lua"),
);
pub(crate) const RENEW: &str = concat!(
  include_str!("renew_lease.lua"),
  "\n",
  include_str!("renew.lua"),
);
pub(crate) const RELEASE: &str = include_str!("release.lua");

/// Определяет Lua-функцию `acquire_lease(key, fence_key, owner_value, ttl_ms)`.
///
/// `fence_key` — ключ lease с суффиксом `:fence`, `owner_value` — `lease:<owner>`.
/// Возвращает `{1, fence}`, `{2, remaining_ms}` или Redis error reply.
/// `fence` — точная десятичная строка, `remaining_ms` — целое число.
/// Вызывающий скрипт должен вернуть error reply или результат «занято» до
/// выполнения защищённых записей. TTL должен быть проверен до вызова функции.
pub const ACQUIRE_LEASE: &str = include_str!("acquire_lease.lua");

/// Определяет Lua-функцию `renew_lease(key, token_value, ttl_ms)`.
///
/// `token_value` формируется через [`crate::lease_value`]. Возвращает `1`
/// при продлении и `0` при потере владения. Защищённые записи допустимы
/// только после `1`. TTL должен быть проверен до вызова функции.
pub const RENEW_LEASE: &str = include_str!("renew_lease.lua");

/// Фрагмент `holds_lease(key, token_value)` для встраивания проверки владения
/// в чужой атомарный скрипт. См. `holds_lease.lua`.
pub const HOLDS_LEASE: &str = include_str!("holds_lease.lua");
