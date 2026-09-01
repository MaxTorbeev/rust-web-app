//! JetStream adapter for the transport-independent `event-bus` core.
//!
//! This crate owns NATS subject mapping, outbound publication and translation
//! of incoming processing outcomes into JetStream delivery settlement.
//! Application composition and consumer task supervision remain outside the
//! adapter.

mod consumer;
mod error;
mod publisher;
mod subject;
mod subject_config;

pub mod health;

pub use consumer::{
  JetStreamConsumerError, JetStreamIncomingConsumer, JetStreamIncomingConsumerConfig,
  JetStreamIncomingConsumerConfigError,
};
pub use publisher::JetStreamEventPublisher;
pub use subject_config::JetStreamSubjectConfig;

#[cfg(test)]
mod tests;
