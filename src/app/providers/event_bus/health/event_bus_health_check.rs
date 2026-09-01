use event_bus_jetstream::health::HealthCheck as ConsumerHealthCheck;
use nats_client::health::HealthCheck as NatsHealthCheck;
use support::health::VerifyHealth;

use super::EventBusHealthState;

/// Verifies the dependencies required by the configured EventBus driver.
#[derive(Clone)]
pub(crate) enum EventBusHealthCheck {
  Disabled,
  JetStream {
    topology: Box<NatsHealthCheck>,
    consumer: ConsumerHealthCheck,
  },
}

impl VerifyHealth for EventBusHealthCheck {
  type Report = EventBusHealthState;

  async fn verify(&self) -> Self::Report {
    match self {
      Self::Disabled => EventBusHealthState::Disabled,
      Self::JetStream { topology, consumer } => {
        let topology = topology.verify().await;
        let consumer = consumer.verify().await;

        EventBusHealthState::JetStream { topology, consumer }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn disabled_check_reports_disabled_for_local_event_bus() {
    let state = EventBusHealthCheck::Disabled.verify().await;

    assert!(matches!(state, EventBusHealthState::Disabled));
  }
}
