//! JetStream publisher adapter for the transport-independent `event-bus` core.
//!
//! This crate owns NATS subject mapping and outbound publication. Event
//! dispatch, consumer settlement and application composition remain outside
//! the publisher interface.

mod config;
mod error;
mod publisher;
mod subject;

pub use config::JetStreamPublisherConfig;
pub use publisher::JetStreamEventPublisher;

#[cfg(test)]
mod tests;
