use redis_client::health::HealthCheck as RedisHealthCheck;
use support::NodeIdentity;
use support::health::VerifyHealth;

use crate::app::providers::EventBusHealthCheck;
use crate::app::version::AppVersion;

use super::{HealthState, TrafficState};

/// Aggregates application metadata and component health checks.
pub(crate) struct HealthCheck {
  version: AppVersion,
  node: NodeIdentity,
  redis: RedisHealthCheck,
  event_bus: EventBusHealthCheck,
}

impl HealthCheck {
  pub(crate) fn new(
    version: AppVersion,
    node: NodeIdentity,
    redis: RedisHealthCheck,
    event_bus: EventBusHealthCheck,
  ) -> Self {
    Self {
      version,
      node,
      redis,
      event_bus,
    }
  }

  pub(crate) const fn version(&self) -> AppVersion {
    self.version
  }

  pub(crate) const fn node(&self) -> &NodeIdentity {
    &self.node
  }

  /// Текущее traffic state приложения.
  ///
  /// Draining пока не реализован, поэтому приложение всегда принимает трафик.
  pub(crate) const fn traffic(&self) -> TrafficState {
    TrafficState::Accepting
  }
}

impl VerifyHealth for HealthCheck {
  type Report = HealthState;

  async fn verify(&self) -> Self::Report {
    let (redis, event_bus) = tokio::join!(self.redis.verify(), self.event_bus.verify(),);

    HealthState::new(
      self.version,
      self.node.clone(),
      self.traffic(),
      redis,
      event_bus,
    )
  }
}
