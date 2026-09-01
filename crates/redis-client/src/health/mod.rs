use std::future::Future;
use std::sync::Arc;

use support::health::{HealthReport, VerifyHealth};

use crate::{RedisClient, RedisClientError};

/// Performs a health verification through an existing Redis client.
#[derive(Clone)]
pub struct HealthCheck {
  client: Arc<RedisClient>,
}

impl HealthCheck {
  pub fn new(client: Arc<RedisClient>) -> Self {
    Self { client }
  }
}

/// Current result of a Redis health verification.
#[derive(Debug)]
pub enum HealthState {
  Up,
  Down(RedisClientError),
}

impl HealthReport for HealthState {
  fn is_healthy(&self) -> bool {
    matches!(self, Self::Up)
  }
}

impl VerifyHealth for HealthCheck {
  type Report = HealthState;

  fn verify(&self) -> impl Future<Output = Self::Report> + Send + '_ {
    async move {
      match self.client.ping().await {
        Ok(()) => HealthState::Up,
        Err(error) => HealthState::Down(error),
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use std::io;

  use super::*;

  #[test]
  fn reports_up_as_healthy() {
    assert!(HealthState::Up.is_healthy());
  }

  #[test]
  fn reports_down_as_unhealthy() {
    let error = RedisClientError::connection(io::Error::other("Redis is unavailable"));

    assert!(!HealthState::Down(error).is_healthy());
  }
}
