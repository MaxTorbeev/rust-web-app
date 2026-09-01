use redis_client::health::HealthState as RedisHealthState;
use support::health::HealthReport;

use crate::app::providers::EventBusHealthState;
use crate::app::version::AppVersion;

/// Snapshot produced by the application health verification.
#[derive(Debug)]
pub(crate) struct HealthState {
  version: AppVersion,
  redis: RedisHealthState,
  event_bus: EventBusHealthState,
}

impl HealthState {
  pub(super) const fn new(
    version: AppVersion,
    redis: RedisHealthState,
    event_bus: EventBusHealthState,
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

  pub(crate) const fn redis(&self) -> &RedisHealthState {
    &self.redis
  }

  pub(crate) const fn event_bus(&self) -> &EventBusHealthState {
    &self.event_bus
  }
}

impl HealthReport for HealthState {
  fn is_healthy(&self) -> bool {
    self.redis.is_healthy() && self.event_bus.is_healthy()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn combines_version_with_component_health() {
    let state = HealthState::new(
      AppVersion::CURRENT,
      RedisHealthState::Up,
      EventBusHealthState::Disabled,
    );

    assert_eq!(state.version(), AppVersion::CURRENT);
    assert!(matches!(state.redis(), RedisHealthState::Up));
    assert!(matches!(state.event_bus(), EventBusHealthState::Disabled));
    assert!(state.is_healthy());
  }
}
