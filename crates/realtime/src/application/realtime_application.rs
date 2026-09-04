use crate::{
  ApplicationId, ApplicationSettings, ChannelRouter, Connection, ConnectionId, PresenceService,
  ProtocolMessage,
};
use auth::{TokenAccessIssuer, TokenAccessVerifier, VerifiedToken};
use std::sync::Arc;
use support::NodeInstance;

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
      router: Arc::new(ChannelRouter::new()),
      presence: Arc::new(PresenceHub::new()),
    }
  }

  pub fn router(&self) -> &ChannelRouter {
    &self.router.as_ref()
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
  pub async fn disconnect_connection(&self, connection_id: &ConnectionId) {
    let leaves = self.presence().disconnect(connection_id).await;

    self.router().disconnect(connection_id).await;

    for (channel, presence) in leaves {
      if let Err(error) = self
        .router()
        .broadcast(
          &channel,
          ProtocolMessage::presence(&channel, vec![presence]),
        )
        .await
      {
        tracing::error!(%error, %channel, "failed to broadcast disconnected presence leave");
      }
    }
  }
}
