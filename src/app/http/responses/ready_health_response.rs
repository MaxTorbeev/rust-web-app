use redis_client::health::HealthState as RedisHealthState;
use serde::Serialize;
use support::health::HealthReport;

use crate::app::health::HealthState;

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
      checks: HealthChecksResponse {
        redis: match state.redis() {
          RedisHealthState::Up => CheckStatus::Up,
          RedisHealthState::Down(_) => CheckStatus::Down,
        },
      },
    }
  }
}

#[derive(Serialize)]
struct HealthChecksResponse {
  redis: CheckStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ReadyStatus {
  Ready,
  NotReady,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum CheckStatus {
  Up,
  Down,
}
