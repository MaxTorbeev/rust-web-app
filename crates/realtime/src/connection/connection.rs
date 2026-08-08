use auth::{VerifiedToken};
use support::timestamp::Timestamp;
use crate::{ApplicationId, ApplicationSettings, ConnectionId, RealtimeApplication};

pub struct Connection {
  pub id: ConnectionId,
  application_id: ApplicationId,
  connection_key: String,
  pub authorization: VerifiedToken,
  pub connected_at: Timestamp,
  settings: ApplicationSettings
}

impl Connection {
  pub(crate) fn new(application: &RealtimeApplication, authorization: VerifiedToken) -> Self {
    Self {
      id: ConnectionId::generate(),
      application_id: application.id.clone(),
      connection_key: uuid::Uuid::new_v4().to_string(),
      authorization,
      connected_at: Timestamp::now(),
      settings: application.settings.clone(),
    }
  }

  pub fn application_id(&self) -> &ApplicationId {
    &self.application_id
  }
  pub fn connection_key(&self) -> &str {
    &self.connection_key
  }

  pub fn client_id(&self) -> Option<&str> {
    self.authorization.client_id.as_deref()
  }

  pub fn authorization(&self) -> &VerifiedToken {
    &self.authorization
  }

  pub fn settings(&self) -> &ApplicationSettings {
    &self.settings
  }
}