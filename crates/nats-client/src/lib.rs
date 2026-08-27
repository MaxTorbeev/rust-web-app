//! Generic NATS JetStream transport primitives.
//!
//! Domain events, processing policy, deduplication and application runtime
//! supervision intentionally remain outside this crate.
//!
//! Универсальные транспортные примитивы NATS JetStream. Доменные события,
//! политика обработки, дедупликация и контроль фоновых задач приложения
//! намеренно остаются за пределами этого crate.

mod client;
mod config;
mod error;
mod message;
mod publish_ack;
mod publish_message;
mod subscription;
mod validation;

pub use client::NatsClient;
pub use config::{ConsumerConfig, NatsConfig, StreamConfig, StreamLimits};
pub use error::{
    AckError, ConnectError, ConsumerConfigError, MessageMetadataError, NatsConfigError,
    PublishError, PublishMessageError, ReceiveError, StreamConfigError, StreamLimitsError,
    StreamSetupError, SubscribeError,
};
pub use message::NatsMessage;
pub use publish_ack::PublishAck;
pub use publish_message::PublishMessage;
pub use subscription::NatsSubscription;

#[cfg(test)]
mod tests;
