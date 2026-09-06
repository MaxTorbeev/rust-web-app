use serde::Serialize;

use crate::app::version::AppVersion;

use super::{HEALTH_SCHEMA_VERSION, ReleaseResponse};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LiveHealthResponse {
  schema_version: u8,
  status: LiveStatus,
  release: ReleaseResponse,
}

impl From<AppVersion> for LiveHealthResponse {
  fn from(version: AppVersion) -> Self {
    Self {
      schema_version: HEALTH_SCHEMA_VERSION,
      status: LiveStatus::Alive,
      release: ReleaseResponse::from(version),
    }
  }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum LiveStatus {
  Alive,
}
