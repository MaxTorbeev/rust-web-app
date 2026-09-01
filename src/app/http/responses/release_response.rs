use serde::Serialize;

use crate::app::version::AppVersion;

#[derive(Serialize)]
pub(crate) struct ReleaseResponse {
  version: &'static str,
  revision: &'static str,
}

impl From<AppVersion> for ReleaseResponse {
  fn from(version: AppVersion) -> Self {
    Self {
      version: version.version(),
      revision: version.revision(),
    }
  }
}
