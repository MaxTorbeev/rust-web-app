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
  realtime: std::sync::Arc<realtime::Realtime>,
}

impl HealthCheck {
  pub(crate) fn new(
    version: AppVersion,
    node: NodeIdentity,
    redis: RedisHealthCheck,
    event_bus: EventBusHealthCheck,
    realtime: std::sync::Arc<realtime::Realtime>,
  ) -> Self {
    Self {
      version,
      node,
      redis,
      event_bus,
      realtime,
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
  /// Redis Presence снимает готовность при остановке обязательных workers.
  pub(crate) fn traffic(&self) -> TrafficState {
    if self.realtime.is_ready() {
      TrafficState::Accepting
    } else {
      TrafficState::Draining
    }
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
