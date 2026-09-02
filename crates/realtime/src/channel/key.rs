use crate::ApplicationId;

#[derive(Debug, Clone)]
pub struct ChannelKey {
  pub application_id: ApplicationId,
  pub channel: String,
}
