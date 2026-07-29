use std::sync::Arc;
use auth::{TokenAccessVerifier, TokenVerifyError, VerifiedToken};
use crate::{ApplicationId, ApplicationKeyName, ApplicationRegistry, RealtimeApplication, RealtimeConfig};

pub struct Realtime {
  applications: ApplicationRegistry,
}

pub struct RealtimeAccess {
  pub application: Arc<RealtimeApplication>,
  pub token: VerifiedToken
}

pub enum RealtimeAuthError {
  InvalidKeyName(String),
  UnknownApplication(ApplicationId),
  TokenVerification(TokenVerifyError),
}

impl From<TokenVerifyError> for RealtimeAuthError {
  fn from(err: TokenVerifyError) -> Self {
    Self::TokenVerification(err)
  }
}

impl Realtime {
  pub fn from_config(config: RealtimeConfig) -> Self {
    Self {
      applications: ApplicationRegistry::from_config(config),
    }
  }

  pub fn application(&self, application_id: &ApplicationId) -> Option<Arc<RealtimeApplication>> {
    self.applications.get(application_id)
  }

  pub fn verify_access_token(&self, access_token: &str) -> Result<RealtimeAccess, RealtimeAuthError> {
    let key_name = TokenAccessVerifier::unverified_key_id(access_token)?;

    let application_key_name = key_name
      .parse::<ApplicationKeyName>()
      .map_err(|_| {
        RealtimeAuthError::InvalidKeyName(key_name.clone())
      })?;

    let application_id = application_key_name.application_id();

    let application = self
      .application(application_id)
      .ok_or_else(|| {
        RealtimeAuthError::UnknownApplication(
          application_id.clone()
        )
      })?;

    let token = application
      .token_verifier
      .verify(access_token)?;

    Ok(RealtimeAccess {
      application,
      token,
    })
  }
}