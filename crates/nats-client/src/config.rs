use std::time::Duration;

use async_nats::jetstream::stream::{Config as DriverStreamConfig, StorageType};

use async_nats::jetstream::consumer::{
  AckPolicy,
  DeliverPolicy,
  pull::Config as DriverConsumerConfig,
};

#[derive(Debug, Clone)]
pub struct NatsConfig {
  pub servers: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StreamConfig {
  pub name: String,
  pub subjects: Vec<String>,
  pub max_messages: i64,
  pub max_message_size: i32,
  pub max_bytes: i64,
  pub max_age: Duration,
  pub replicas: usize,
}

#[derive(Debug, Clone)]
pub struct ConsumerConfig {
  pub stream_name: String,
  pub durable_name: String,
  pub filter_subject: String,
}

impl NatsConfig {
  pub fn new(servers: Vec<String>) -> Self {
    Self {
      servers
    }
  }
}

impl StreamConfig {
  pub(crate) fn into_driver_config(self) -> DriverStreamConfig {
    DriverStreamConfig {
      name: self.name,
      subjects: self.subjects,
      max_messages: self.max_messages,
      max_message_size: self.max_message_size,
      max_bytes: self.max_bytes,
      max_age: self.max_age,
      storage: StorageType::File,
      num_replicas: self.replicas,
      ..Default::default()
    }
  }
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