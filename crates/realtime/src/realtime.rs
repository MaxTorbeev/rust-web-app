use crate::{
  ApplicationId, ApplicationKeyName, ApplicationRegistry, RealtimeApplication, RealtimeConfig,
};
use auth::{
  TokenAccessVerifier, TokenCapability, TokenIssueError, TokenVerifyError, VerifiedToken,
};
use std::sync::Arc;
use support::NodeInstance;

pub struct Realtime {
  applications: ApplicationRegistry,
}

pub struct RealtimeAccess {
  pub application: Arc<RealtimeApplication>,
  pub token: VerifiedToken,
}

pub enum RealtimeAuthError {
  InvalidKeyName(String),
  UnknownApplication(ApplicationId),
  TokenVerification(TokenVerifyError),
  TokenIssuance(TokenIssueError),
}

impl From<TokenVerifyError> for RealtimeAuthError {
  fn from(err: TokenVerifyError) -> Self {
    Self::TokenVerification(err)
  }
}

impl From<TokenIssueError> for RealtimeAuthError {
  fn from(error: TokenIssueError) -> Self {
    Self::TokenIssuance(error)
  }
}

impl Realtime {
  pub fn from_config(config: RealtimeConfig, node_instance: NodeInstance) -> Self {
    Self {
      applications: ApplicationRegistry::from_config(config, node_instance),
    }
  }

  pub fn application(&self, application_id: &ApplicationId) -> Option<Arc<RealtimeApplication>> {
    self.applications.get(application_id)
  }

  pub fn verify_access_token(
    &self,
    access_token: &str,
  ) -> Result<RealtimeAccess, RealtimeAuthError> {
    let key_name = TokenAccessVerifier::unverified_key_id(access_token)?;

    let application_key_name = key_name
      .parse::<ApplicationKeyName>()
      .map_err(|_| RealtimeAuthError::InvalidKeyName(key_name.clone()))?;

    let application_id = application_key_name.application_id();

    // TODO(security): WARNING: the registry currently resolves only one static key per application.
    // Resolve `key_id` through a product-key registry to check status, revocation and key permissions.
    let application = self
      .application(application_id)
      .ok_or_else(|| RealtimeAuthError::UnknownApplication(application_id.clone()))?;

    let token = application.token_verifier.verify(access_token)?;

    Ok(RealtimeAccess { application, token })
  }

  pub fn issue_access_token(
    &self,
    application_id: &ApplicationId,
    client_id: String,
    capability: &TokenCapability,
    ttl_seconds: u64,
  ) -> Result<String, RealtimeAuthError> {
    let application = self
      .application(application_id)
      .ok_or_else(|| RealtimeAuthError::UnknownApplication(application_id.clone()))?;

    let token = application
      .token_issuer
      .issue(Some(client_id), capability, ttl_seconds)?;

    Ok(token)
  }
}
