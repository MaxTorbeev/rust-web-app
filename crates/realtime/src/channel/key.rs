use crate::ApplicationId;
use serde::{Deserialize, Serialize};

/// Канал и приложение, к которым подключается соединение.
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
pub struct ChannelKey {
  pub application_id: ApplicationId,
  pub channel: String,
}

impl ChannelKey {
  pub fn new(application_id: ApplicationId, channel: impl Into<String>) -> Self {
    Self {
      application_id,
      channel: channel.into(),
    }
  }
}
