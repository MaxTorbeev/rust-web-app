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
//! - формат значения ([`lease_value`]) и Lua-фрагмент [`LUA_HOLDS_LEASE`] — когда
//!   проверку владения нужно встроить в чужой атомарный скрипт (state transition
//!   хранилища проверяет lease экземпляра ноды в той же операции, что меняет
//!   состояние).
//!
//! Все операции атомарны на стороне Redis и опираются на его TTL, а не на часы
//! нод: две ноды с расходящимися часами одинаково видят, истёк lease или нет.

// Публичный API: всё содержимое этих модулей реэкспортируется ниже.
mod error;
mod identity;
mod lease;
mod outcome;

// Внутреннее: раскладка lease в Redis и Lua-скрипты. Наружу отдаются только две
// вещи, которые воспроизводят чужие скрипты, — форма значения и фрагмент
// `holds_lease`.
mod protocol;
mod scripts;

pub use error::RedisLeaseError;
pub use identity::{LeaseKey, LeaseOwner, LeaseToken};
pub use lease::RedisLease;
pub use outcome::{AcquireOutcome, Fence, ReleaseOutcome, RenewOutcome};
pub use protocol::lease_value;
pub use scripts::HOLDS_LEASE as LUA_HOLDS_LEASE;

#[cfg(test)]
mod tests;
