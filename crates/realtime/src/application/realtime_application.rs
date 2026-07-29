use std::sync::Arc;
use auth::{TokenAccessIssuer, TokenAccessVerifier};
use crate::{ApplicationId, ChannelHub, PresenceHub};

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
}