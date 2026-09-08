//! Всё Lua крейта: операции lease и фрагменты для чужих скриптов.

pub(crate) const ACQUIRE: &str = include_str!("acquire.lua");
pub(crate) const RENEW: &str = include_str!("renew.lua");
pub(crate) const RELEASE: &str = include_str!("release.lua");

/// Фрагмент `holds_lease(key, owner_value)` для встраивания проверки владения
/// в чужой атомарный скрипт. См. `holds_lease.lua`.
pub const HOLDS_LEASE: &str = include_str!("holds_lease.lua");
