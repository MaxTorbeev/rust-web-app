use support::health::HealthReport;

/// Current lifecycle state of the incoming JetStream consumer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HealthState {
  /// The worker exists but has not polled its delivery stream yet.
  Starting,
  /// The worker has entered the receive loop and can await deliveries.
  Running,
  /// The receive loop terminated with an error.
  Failed,
  /// The worker was dropped or cancelled without reporting an error.
  Stopped,
}

impl HealthReport for HealthState {
  fn is_healthy(&self) -> bool {
    matches!(self, Self::Running)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn only_running_consumer_is_healthy() {
    assert!(!HealthState::Starting.is_healthy());
    assert!(HealthState::Running.is_healthy());
    assert!(!HealthState::Failed.is_healthy());
    assert!(!HealthState::Stopped.is_healthy());
  }
}
