use std::sync::Arc;

use support::health::VerifyHealth;

use crate::{ConsumerConfig, NatsClient, StreamConfig};

use super::HealthState;

/// Performs a read-only health verification of configured JetStream topology.
#[derive(Clone)]
pub struct HealthCheck {
  client: Arc<NatsClient>,
  stream: StreamConfig,
  consumer: ConsumerConfig,
}

impl HealthCheck {
  pub fn new(client: Arc<NatsClient>, stream: StreamConfig, consumer: ConsumerConfig) -> Self {
    Self {
      client,
      stream,
      consumer,
    }
  }
}

impl VerifyHealth for HealthCheck {
  type Report = HealthState;

  async fn verify(&self) -> Self::Report {
    match self
      .client
      .verify_topology(&self.stream, &self.consumer)
      .await
    {
      Ok(()) => HealthState::Up,
      Err(error) => HealthState::Down(error),
    }
  }
}
