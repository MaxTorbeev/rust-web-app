use nats_client::health::HealthState as NatsHealthState;
use support::health::HealthReport;

/// Current health of the dependencies required by the EventBus driver.
#[derive(Debug)]
pub(crate) enum EventBusHealthState {
  Disabled,
  JetStream(NatsHealthState),
}

impl HealthReport for EventBusHealthState {
  fn is_healthy(&self) -> bool {
    match self {
      Self::Disabled => true,
      Self::JetStream(state) => state.is_healthy(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn disabled_jetstream_is_healthy_for_local_event_bus() {
    assert!(EventBusHealthState::Disabled.is_healthy());
  }

  #[test]
  fn healthy_jetstream_is_healthy_for_distributed_event_bus() {
    let state = EventBusHealthState::JetStream(NatsHealthState::Up);

    assert!(state.is_healthy());
  }
}
