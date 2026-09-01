use event_bus_jetstream::health::HealthState as ConsumerHealthState;
use nats_client::health::HealthState as NatsHealthState;
use support::health::HealthReport;

/// Current health of the dependencies required by the EventBus driver.
#[derive(Debug)]
pub(crate) enum EventBusHealthState {
  Disabled,
  JetStream {
    topology: NatsHealthState,
    consumer: ConsumerHealthState,
  },
}

impl HealthReport for EventBusHealthState {
  fn is_healthy(&self) -> bool {
    match self {
      Self::Disabled => true,
      Self::JetStream { topology, consumer } => topology.is_healthy() && consumer.is_healthy(),
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
    let state = EventBusHealthState::JetStream {
      topology: NatsHealthState::Up,
      consumer: ConsumerHealthState::Running,
    };

    assert!(state.is_healthy());
  }

  #[test]
  fn non_running_consumer_is_unhealthy_for_distributed_event_bus() {
    for consumer in [
      ConsumerHealthState::Starting,
      ConsumerHealthState::Failed,
      ConsumerHealthState::Stopped,
    ] {
      let state = EventBusHealthState::JetStream {
        topology: NatsHealthState::Up,
        consumer,
      };

      assert!(!state.is_healthy());
    }
  }
}
