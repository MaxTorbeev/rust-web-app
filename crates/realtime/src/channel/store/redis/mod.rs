//! Redis-адаптер состояния каналов.
//!
//! На этом этапе определена только схема ключей. Store и Lua transitions
//! будут реализованы отдельно; runtime продолжает использовать memory store.
//! Схема и жизненный цикл данных описаны в `docs/redis-store-schema.md`.

mod keys;
mod protocol;
mod scripts;
mod store;
mod node_lease;

pub use keys::RedisKeys;
pub use node_lease::*;

#[cfg(test)]
mod tests;
