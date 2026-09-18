//! Lease в Redis: кто сейчас владеет ресурсом, с автоматической потерей права
//! по истечении TTL и защитой от «очнувшегося» старого владельца.
//!
//! Единица владения — период: захват выдаёт [`LeaseToken`] (ключ, владелец,
//! fence), и именно его предъявляют `renew`, `release` и проверка `holds_lease`.
//! Владелец один во всех своих периодах, token — у каждого свой, поэтому
//! отложенный повтор release или работа из прошлого периода не задевают новый
//! lease того же владельца.
//!
//! Два уровня использования:
//!
//! - операции [`RedisLease::acquire`], [`RedisLease::renew`],
//!   [`RedisLease::release`] — когда lease нужен сам по себе;
//! - формат значения ([`lease_value`]) и Lua-фрагменты [`LUA_ACQUIRE_LEASE`],
//!   [`LUA_RENEW_LEASE`], [`LUA_HOLDS_LEASE`] — когда захват, продление или
//!   проверку владения нужно встроить в чужой атомарный скрипт.
//!
//! Фрагменты определяют локальные Lua-функции: к ним добавляют код вызывающего
//! скрипта и отправляют полученную строку одним вызовом Redis. Проверки входных
//! данных и типов дополнительных ключей выполняют до первой записи: ошибка
//! Lua не откатывает уже выполненные изменения.
//!
//! Все операции атомарны на стороне Redis и опираются на его TTL, а не на часы
//! нод: две ноды с расходящимися часами одинаково видят, истёк lease или нет.

// Публичный API: всё содержимое этих модулей реэкспортируется ниже.
mod error;
mod identity;
mod lease;
mod outcome;

// Раскладка lease и Lua-скрипты. Для составных скриптов экспортируются
// формат значения token и определения функций захвата, продления и проверки.
mod protocol;
mod scripts;

pub use error::RedisLeaseError;
pub use identity::{LeaseKey, LeaseOwner, LeaseToken};
pub use lease::RedisLease;
pub use outcome::{AcquireOutcome, Fence, ReleaseOutcome, RenewOutcome};
pub use protocol::lease_value;
pub use scripts::{
  ACQUIRE_LEASE as LUA_ACQUIRE_LEASE, HOLDS_LEASE as LUA_HOLDS_LEASE,
  RENEW_LEASE as LUA_RENEW_LEASE,
};

#[cfg(test)]
mod tests;
