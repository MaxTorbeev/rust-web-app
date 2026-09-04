use crate::connection::ConnectionActor;
use crate::{ApplicationId, ApplicationSettings, ConnectionId, RealtimeApplication};
use auth::VerifiedToken;
use support::{NodeInstance, timestamp::Timestamp};

pub struct Connection {
  pub id: ConnectionId,
  application_id: ApplicationId,
  node_instance: NodeInstance,
  connection_key: String,
  pub authorization: VerifiedToken,
  pub connected_at: Timestamp,
  settings: ApplicationSettings,
}

impl Connection {
  pub(crate) fn new(application: &RealtimeApplication, authorization: VerifiedToken) -> Self {
    Self {
      id: ConnectionId::generate(),
      application_id: application.id.clone(),
      node_instance: application.node_instance().clone(),
      connection_key: uuid::Uuid::new_v4().to_string(),
      authorization,
      connected_at: Timestamp::now(),
      settings: application.settings.clone(),
    }
  }

  pub fn application_id(&self) -> &ApplicationId {
    &self.application_id
  }

  pub fn node_instance(&self) -> &NodeInstance {
    &self.node_instance
  }

  pub fn actor(&self) -> ConnectionActor {
    ConnectionActor {
      application_id: self.application_id.clone(),
      connection_id: self.id.clone(),
      node_instance: self.node_instance.clone(),
    }
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{ConnectionDetails, ProtocolMessage};
  use auth::{TokenAccessIssuer, TokenAccessVerifier};
  use support::{BootGeneration, NodeId};

  fn test_node_instance() -> NodeInstance {
    NodeInstance::new(
      NodeId::try_new("test-node").expect("test node id must be valid"),
      BootGeneration::generate(),
      Timestamp::from_millis(1_700_000_000_000),
    )
  }

  fn test_application(id: &str, settings: ApplicationSettings) -> RealtimeApplication {
    let mut application = RealtimeApplication::new(
      ApplicationId::new(id),
      test_node_instance(),
      TokenAccessIssuer::new("test-key", b"test-secret"),
      TokenAccessVerifier::new("test-key", b"test-secret"),
    );

    application.settings = settings;
    application
  }

  fn test_authorization() -> VerifiedToken {
    VerifiedToken {
      client_id: Some("client-123".to_owned()),
      issued_at: 1,
      expires_at: 2,
      capability: r#"{"*": ["subscribe"]}"#
        .parse()
        .expect("test capability must be valid"),
    }
  }

  #[test]
  fn connected_messages_keep_the_connection_key() {
    let application = test_application("application-1", ApplicationSettings::default());
    let connection = application.create_connection(test_authorization());

    let first = ProtocolMessage::connected(&connection);
    let second = ProtocolMessage::connected(&connection);

    let first_key = first
      .connection_details
      .as_ref()
      .expect("CONNECTED must contain connection details")
      .connection_key
      .as_str();
    let second_key = second
      .connection_details
      .as_ref()
      .expect("CONNECTED must contain connection details")
      .connection_key
      .as_str();

    assert_eq!(first_key, connection.connection_key());
    assert_eq!(second_key, first_key);
  }

  #[test]
  fn connection_keeps_the_creating_application_id() {
    let application = test_application("application-1", ApplicationSettings::default());

    let connection = application.create_connection(test_authorization());

    assert_eq!(connection.application_id(), &application.id);
  }

  #[test]
  fn connection_keeps_the_creating_node_instance() {
    let application = test_application("application-1", ApplicationSettings::default());
    let expected = application.node_instance().clone();

    let connection = application.create_connection(test_authorization());

    assert_eq!(connection.node_instance(), &expected);
    assert_eq!(connection.actor().node_instance, expected);
  }

  #[test]
  fn connection_details_use_the_settings_snapshot_from_connect_time() {
    let initial_settings = ApplicationSettings {
      max_message_size: 1_001,
      max_inbound_rate: 1_002,
      max_outbound_rate: 1_003,
      max_frame_size: 1_004,
      connection_state_ttl: 1_005,
      max_idle_interval: 1_006,
    };
    let mut application = test_application("application-1", initial_settings.clone());
    let connection = application.create_connection(test_authorization());

    // Later application changes must not alter an established connection.
    application.settings = ApplicationSettings::default();

    let details = ConnectionDetails::from(&connection);

    assert_eq!(details.max_message_size, initial_settings.max_message_size);
    assert_eq!(details.max_inbound_rate, initial_settings.max_inbound_rate);
    assert_eq!(
      details.max_outbound_rate,
      initial_settings.max_outbound_rate
    );
    assert_eq!(details.max_frame_size, initial_settings.max_frame_size);
    assert_eq!(
      details.connection_state_ttl,
      initial_settings.connection_state_ttl,
    );
    assert_eq!(
      details.max_idle_interval,
      initial_settings.max_idle_interval
    );
  }
}
