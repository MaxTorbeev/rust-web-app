//! JetStream adapter for the transport-independent `event-bus` core.
//!
//! This crate owns NATS subject mapping, outbound publication and translation
//! of incoming processing outcomes into JetStream delivery settlement.
//! Application composition and consumer task supervision remain outside the
//! adapter.

mod config;
mod consumer;
mod error;
mod publisher;
mod subject;

pub use config::JetStreamPublisherConfig;
pub use consumer::{JetStreamConsumerError, JetStreamIncomingConsumer};
pub use publisher::JetStreamEventPublisher;

#[cfg(test)]
mod tests;
