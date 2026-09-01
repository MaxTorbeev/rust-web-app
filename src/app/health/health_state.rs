use redis_client::health::HealthState as RedisHealthState;
use support::health::HealthReport;

use crate::app::version::AppVersion;

/// Snapshot produced by the application health verification.
#[derive(Debug)]
pub(crate) struct HealthState {
  version: AppVersion,
  redis: RedisHealthState,
}

impl HealthState {
  pub(super) const fn new(version: AppVersion, redis: RedisHealthState) -> Self {
    Self { version, redis }
  }

  pub(crate) const fn version(&self) -> AppVersion {
    self.version
  }

  pub(crate) const fn redis(&self) -> &RedisHealthState {
    &self.redis
  }
}

impl HealthReport for HealthState {
  fn is_healthy(&self) -> bool {
    self.redis.is_healthy()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn combines_version_with_redis_health() {
    let state = HealthState::new(AppVersion::CURRENT, RedisHealthState::Up);

    assert_eq!(state.version(), AppVersion::CURRENT);
    assert!(matches!(state.redis(), RedisHealthState::Up));
    assert!(state.is_healthy());
  }
}
