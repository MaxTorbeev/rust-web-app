use redis_client::health::HealthCheck as RedisHealthCheck;
use support::health::VerifyHealth;

use crate::app::providers::EventBusHealthCheck;
use crate::app::version::AppVersion;

use super::HealthState;

/// Aggregates application metadata and component health checks.
pub(crate) struct HealthCheck {
  version: AppVersion,
  redis: RedisHealthCheck,
  event_bus: EventBusHealthCheck,
}

impl HealthCheck {
  pub(crate) fn new(
    version: AppVersion,
    redis: RedisHealthCheck,
    event_bus: EventBusHealthCheck,
  ) -> Self {
    Self {
      version,
      redis,
      event_bus,
    }
  }

  pub(crate) const fn version(&self) -> AppVersion {
    self.version
  }
}

impl VerifyHealth for HealthCheck {
  type Report = HealthState;

  async fn verify(&self) -> Self::Report {
    let (redis, event_bus) = tokio::join!(self.redis.verify(), self.event_bus.verify(),);

    HealthState::new(self.version, redis, event_bus)
  }
}
