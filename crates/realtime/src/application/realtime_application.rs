use crate::{
  ApplicationId, ApplicationSettings, ChannelHub, Connection, ConnectionId, PresenceHub,
  ProtocolMessage,
};
use auth::{TokenAccessIssuer, TokenAccessVerifier, VerifiedToken};
use std::sync::Arc;

pub struct RealtimeApplication {
  pub id: ApplicationId,
  pub(crate) token_issuer: TokenAccessIssuer,
  pub token_verifier: TokenAccessVerifier,
  pub channel_hub: Arc<ChannelHub>,
  pub presence_hub: Arc<PresenceHub>,
  pub settings: ApplicationSettings,
}

impl RealtimeApplication {
  pub fn new(
    id: ApplicationId,
    token_issuer: TokenAccessIssuer,
    token_verifier: TokenAccessVerifier,
  ) -> Self {
    Self {
      id,
      token_issuer,
      token_verifier,
      settings: ApplicationSettings::default(),
      channel_hub: Arc::new(ChannelHub::new()),
      presence_hub: Arc::new(PresenceHub::new()),
    }
  }

  pub fn create_connection(&self, authorization: VerifiedToken) -> Connection {
    Connection::new(self, authorization)
  }

  /// Removes one connection from channel and presence state
  /// and broadcasts the resulting presence leave messages.
  pub async fn disconnect_connection(&self, connection_id: &ConnectionId) {
    let leaves = self.presence_hub.disconnect(connection_id).await;

    self.channel_hub.disconnect(connection_id).await;

    for (channel, presence) in leaves {
      if let Err(error) = self
        .channel_hub
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
