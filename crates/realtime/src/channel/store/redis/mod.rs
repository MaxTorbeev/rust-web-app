//! Redis-адаптер состояния каналов.
//!
//! На этом этапе определена только схема ключей. Store и Lua transitions
//! будут реализованы отдельно; runtime продолжает использовать memory store.
//! Схема и жизненный цикл данных описаны в `docs/redis-store-schema.md`.

mod keys;
mod node;
mod protocol;
mod scripts;
mod store;

pub use keys::RedisKeys;

#[cfg(test)]
mod tests;
