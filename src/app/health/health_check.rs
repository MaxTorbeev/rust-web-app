use std::future::Future;

use redis_client::health::HealthCheck as RedisHealthCheck;
use support::health::VerifyHealth;

use crate::app::version::AppVersion;

use super::HealthState;

/// Aggregates application metadata and component health checks.
pub(crate) struct HealthCheck {
  version: AppVersion,
  redis: RedisHealthCheck,
}

impl HealthCheck {
  pub(crate) fn new(version: AppVersion, redis: RedisHealthCheck) -> Self {
    Self { version, redis }
  }

  pub(crate) const fn version(&self) -> AppVersion {
    self.version
  }
}

impl VerifyHealth for HealthCheck {
  type Report = HealthState;

  fn verify(&self) -> impl Future<Output = Self::Report> + Send + '_ {
    async move {
      let redis = self.redis.verify().await;

      HealthState::new(self.version, redis)
    }
  }
}
