use crate::ApplicationId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChannelKey {
  pub application_id: ApplicationId,
  pub channel: String,
}
