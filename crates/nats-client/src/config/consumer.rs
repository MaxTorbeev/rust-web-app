use std::time::Duration;

use async_nats::jetstream::stream::{Config as DriverStreamConfig, StorageType};

use async_nats::jetstream::consumer::{
  AckPolicy,
  DeliverPolicy,
  pull::Config as DriverConsumerConfig,
};

#[derive(Debug, Clone)]
pub struct ConsumerConfig {
  pub stream_name: String,
  pub durable_name: String,
  pub filter_subject: String,
}

impl ConsumerConfig {
  pub(crate) fn to_driver_config(&self) -> DriverConsumerConfig {
    DriverConsumerConfig {
      durable_name: Some(self.durable_name.clone()),
      filter_subject: self.filter_subject.clone(),
      deliver_policy: DeliverPolicy::New,
      ack_policy: AckPolicy::Explicit,
      ..Default::default()
    }
  }
}