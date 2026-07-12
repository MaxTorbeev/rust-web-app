pub mod client;
pub mod config;

pub use config::RedisConfig;
pub use redis::aio::MultiplexedConnection;

pub use client::{connect, ping};
