use crate::{ApplicationId, ApplicationSettings, ChannelRouter, Connection, ConnectionId, PresenceService, ProtocolMessage};
use auth::{TokenAccessIssuer, TokenAccessVerifier, VerifiedToken};
use std::sync::Arc;

pub struct RealtimeApplication {
  pub id: ApplicationId,
  pub(crate) token_issuer: TokenAccessIssuer,
  pub token_verifier: TokenAccessVerifier,
  pub settings: ApplicationSettings,
  router: Arc<ChannelRouter>,
  presence: PresenceService,
}

impl RealtimeApplication {
  pub fn new(
    id: ApplicationId,
    token_issuer: TokenAccessIssuer,
    token_verifier: TokenAccessVerifier,
    router: Arc<ChannelRouter>,
    presence: PresenceService,
  ) -> Self {
    Self {
      id,
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
