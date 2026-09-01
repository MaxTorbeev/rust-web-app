use std::future::Future;

use nats_client::health::HealthCheck as NatsHealthCheck;
use support::health::VerifyHealth;

use super::EventBusHealthState;

/// Verifies the dependencies required by the configured EventBus driver.
#[derive(Clone)]
pub(crate) enum EventBusHealthCheck {
  Disabled,
  JetStream(NatsHealthCheck),
}

impl VerifyHealth for EventBusHealthCheck {
  type Report = EventBusHealthState;

  fn verify(&self) -> impl Future<Output = Self::Report> + Send + '_ {
    async move {
      match self {
        Self::Disabled => EventBusHealthState::Disabled,
        Self::JetStream(check) => EventBusHealthState::JetStream(check.verify().await),
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
