use event_bus_jetstream::health::HealthState as ConsumerHealthState;
use nats_client::health::HealthState as NatsHealthState;
use redis_client::health::HealthState as RedisHealthState;
use serde::Serialize;
use support::health::HealthReport;

use crate::app::health::HealthState;
use crate::app::providers::EventBusHealthState;

use super::{HEALTH_SCHEMA_VERSION, ReleaseResponse};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReadyHealthResponse {
  schema_version: u8,
  status: ReadyStatus,
  release: ReleaseResponse,
  checks: HealthChecksResponse,
}

impl From<&HealthState> for ReadyHealthResponse {
  fn from(state: &HealthState) -> Self {
    Self {
      schema_version: HEALTH_SCHEMA_VERSION,
      status: if state.is_healthy() {
        ReadyStatus::Ready
      } else {
        ReadyStatus::NotReady
      },
      release: ReleaseResponse::from(state.version()),
      checks: HealthChecksResponse::from(state),
    }
  }
}

#[derive(Serialize)]
struct HealthChecksResponse {
  redis: CheckStatus,
  jetstream: CheckStatus,
  consumer: CheckStatus,
}

impl From<&HealthState> for HealthChecksResponse {
  fn from(state: &HealthState) -> Self {
    let redis = match state.redis() {
      RedisHealthState::Up => CheckStatus::Up,
      RedisHealthState::Down(_) => CheckStatus::Down,
    };

    let (jetstream, consumer) = event_bus_checks(state.event_bus());

    Self {
      redis,
      jetstream,
      consumer,
    }
  }
}

fn event_bus_checks(state: &EventBusHealthState) -> (CheckStatus, CheckStatus) {
  match state {
    EventBusHealthState::Disabled => (CheckStatus::Disabled, CheckStatus::Disabled),
    EventBusHealthState::JetStream { topology, consumer } => {
      let jetstream = match topology {
        NatsHealthState::Up => CheckStatus::Up,
        NatsHealthState::Down(_) => CheckStatus::Down,
      };
      let consumer = match consumer {
        ConsumerHealthState::Running => CheckStatus::Up,
        ConsumerHealthState::Starting
        | ConsumerHealthState::Failed
        | ConsumerHealthState::Stopped => CheckStatus::Down,
      };

      (jetstream, consumer)
    }
  }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum ReadyStatus {
  Ready,
  NotReady,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum CheckStatus {
  Up,
  Down,
  Disabled,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn local_event_bus_disables_jetstream_and_consumer_checks() {
    assert_eq!(
      event_bus_checks(&EventBusHealthState::Disabled),
      (CheckStatus::Disabled, CheckStatus::Disabled)
    );
  }

  #[test]
  fn running_consumer_with_healthy_topology_is_up() {
    let state = EventBusHealthState::JetStream {
      topology: NatsHealthState::Up,
      consumer: ConsumerHealthState::Running,
    };

    assert_eq!(event_bus_checks(&state), (CheckStatus::Up, CheckStatus::Up));
  }

  #[test]
  fn non_running_consumer_is_down() {
    for consumer in [
      ConsumerHealthState::Starting,
      ConsumerHealthState::Failed,
      ConsumerHealthState::Stopped,
    ] {
      let state = EventBusHealthState::JetStream {
        topology: NatsHealthState::Up,
        consumer,
      };

      assert_eq!(
        event_bus_checks(&state),
        (CheckStatus::Up, CheckStatus::Down)
      );
    }
  }
}
