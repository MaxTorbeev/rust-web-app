use crate::{ApplicationId, ApplicationSettings, ChannelRouter, Connection, ConnectionCleanupError, ConnectionId, PresenceError, PresenceService, ProtocolMessage};
use auth::{TokenAccessIssuer, TokenAccessVerifier, VerifiedToken};
use std::sync::Arc;
use support::NodeInstance;
use support::timestamp::Timestamp;
use crate::connection::DisconnectConnectionCommand;

pub struct RealtimeApplication {
  pub id: ApplicationId,
  node_instance: NodeInstance,
  pub(crate) token_issuer: TokenAccessIssuer,
  pub token_verifier: TokenAccessVerifier,
  pub settings: ApplicationSettings,
  router: Arc<ChannelRouter>,
  presence: PresenceService,
}

impl RealtimeApplication {
  pub fn new(
    id: ApplicationId,
    node_instance: NodeInstance,
    token_issuer: TokenAccessIssuer,
    token_verifier: TokenAccessVerifier,
    router: Arc<ChannelRouter>,
    presence: PresenceService,
  ) -> Self {
    Self {
      id,
      node_instance,
      token_issuer,
      token_verifier,
      settings: ApplicationSettings::default(),
      router,
      presence,
    }
  }

  pub fn router(&self) -> &ChannelRouter {
    self.router.as_ref()
  }

  pub fn presence(&self) -> &PresenceService {
    &self.presence
  }

  pub fn node_instance(&self) -> &NodeInstance {
    &self.node_instance
  }

  pub fn create_connection(&self, authorization: VerifiedToken) -> Connection {
    Connection::new(self, authorization)
  }

  /// Removes one connection from channel and presence state
  /// and broadcasts the resulting presence leave messages.
  pub async fn disconnect_connection(&self, connection: &Connection) -> Result<(), ConnectionCleanupError> {
    // Сначала исключаем закрывающееся соединение из локальной доставки.
    self.router().disconnect(&connection.id).await;

    self
      .presence()
      .disconnect(DisconnectConnectionCommand {
        actor: connection.actor(),
        request_time: Timestamp::now(),
      })
      .await?;

    Ok(())
  }
}
