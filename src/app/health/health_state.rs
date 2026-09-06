use redis_client::health::HealthState as RedisHealthState;
use support::NodeIdentity;
use support::health::HealthReport;

use crate::app::providers::EventBusHealthState;
use crate::app::version::AppVersion;

use super::TrafficState;

/// Snapshot produced by the application health verification.
#[derive(Debug)]
pub(crate) struct HealthState {
  version: AppVersion,
  node: NodeIdentity,
  traffic: TrafficState,
  redis: RedisHealthState,
  event_bus: EventBusHealthState,
}

impl HealthState {
  pub(super) const fn new(
    version: AppVersion,
    node: NodeIdentity,
    traffic: TrafficState,
    redis: RedisHealthState,
    event_bus: EventBusHealthState,
  ) -> Self {
    Self {
      version,
      node,
      traffic,
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

  pub(crate) const fn traffic(&self) -> TrafficState {
    self.traffic
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
    self.traffic.is_accepting() && self.redis.is_healthy() && self.event_bus.is_healthy()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use support::{DeploymentSlot, NodeId};

  fn test_node() -> NodeIdentity {
    NodeIdentity::new(
      NodeId::try_new("realtime-test").expect("test node id must be valid"),
      DeploymentSlot::Single,
    )
  }

  #[test]
  fn combines_version_with_component_health() {
    let state = HealthState::new(
      AppVersion::CURRENT,
      test_node(),
      TrafficState::Accepting,
      RedisHealthState::Up,
      EventBusHealthState::Disabled,
    );

    assert_eq!(state.version(), AppVersion::CURRENT);
    assert_eq!(state.node().id().as_str(), "realtime-test");
    assert_eq!(state.node().slot(), DeploymentSlot::Single);
    assert!(matches!(state.redis(), RedisHealthState::Up));
    assert!(matches!(state.event_bus(), EventBusHealthState::Disabled));
    assert!(state.is_healthy());
  }

  #[test]
  fn draining_node_is_not_ready_even_with_healthy_components() {
    let state = HealthState::new(
      AppVersion::CURRENT,
      test_node(),
      TrafficState::Draining,
      RedisHealthState::Up,
      EventBusHealthState::Disabled,
    );

    assert!(!state.is_healthy());
  }
}
