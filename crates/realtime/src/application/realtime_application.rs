use std::sync::Arc;
use auth::{TokenAccessIssuer, TokenAccessVerifier};
use crate::{ApplicationId, ChannelHub, ConnectionId, PresenceHub, ProtocolMessage};

pub struct RealtimeApplication {
  pub id: ApplicationId,
  pub(crate) token_issuer: TokenAccessIssuer,
  pub token_verifier: TokenAccessVerifier,
  pub channel_hub: Arc<ChannelHub>,
  pub presence_hub: Arc<PresenceHub>,
}

impl RealtimeApplication {
  pub fn new(
    id: ApplicationId,
    token_issuer: TokenAccessIssuer,
    token_verifier: TokenAccessVerifier
  ) -> Self {
    Self {
      id,
      token_issuer,
      token_verifier,
      channel_hub: Arc::new(ChannelHub::new()),
      presence_hub: Arc::new(PresenceHub::new()),
    }
  }

  /// Removes one connection from channel and presence state
  /// and broadcasts the resulting presence leave messages.
  pub async fn disconnect_connection(
    &self,
    connection_id: &ConnectionId,
  ) {
    let leaves = self
      .presence_hub
      .disconnect(connection_id)
      .await;

    self
      .channel_hub
      .disconnect(connection_id)
      .await;

    for (channel, presence) in leaves {
      self
        .channel_hub
        .broadcast(
          &channel,
          ProtocolMessage::presence(
            &channel,
            vec![presence],
          ),
        )
        .await;
    }
  }
}