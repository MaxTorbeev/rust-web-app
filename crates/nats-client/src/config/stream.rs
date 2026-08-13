use std::time::Duration;
use async_nats::jetstream::stream::{Config as DriverStreamConfig, StorageType};

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
