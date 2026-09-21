//! Redis-адаптер состояния каналов.
//!
//! Точный Presence: node lease, store transitions, outbox publisher и reaper.
//! Включается через PRESENCE_STORE_DRIVER=redis совместно с JetStream.
//! Схема и жизненный цикл данных описаны в `docs/redis-store-schema.md`.

mod keys;
mod member;
mod node;
mod outbox;
mod protocol;
mod reaper;
mod request;
mod response;
mod runtime;
mod scripts;
mod store;
mod traits;

pub use keys::RedisKeys;
pub use node::{NodeClaimOutcome, NodeLease, NodeLeaseError};
pub use runtime::RedisPresenceRuntime;
pub use store::RedisChannelStore;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod integration_tests;
