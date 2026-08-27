use crate::{ApplicationId, RealtimeApplication, RealtimeConfig};
use auth::{TokenAccessIssuer, TokenAccessVerifier};
use std::collections::HashMap;
use std::sync::Arc;

pub struct ApplicationRegistry {
  applications: HashMap<ApplicationId, Arc<RealtimeApplication>>,
}

impl ApplicationRegistry {
  pub fn new() -> Self {
    Self {
      applications: HashMap::new(),
    }
  }

  pub fn from_config(config: RealtimeConfig) -> Self {
    let RealtimeConfig {
      application_id,
      key_name,
      key_secret,
    } = config;

    let token_verified = TokenAccessVerifier::new(key_name.clone(), key_secret.as_bytes());

    let token_issuer = TokenAccessIssuer::new(key_name.clone(), key_secret.as_bytes());

    let application = RealtimeApplication::new(application_id, token_issuer, token_verified);

    let mut registry = Self::new();

    let previous = registry.insert(application);

    debug_assert!(previous.is_none());

    registry
  }

  pub fn insert(&mut self, application: RealtimeApplication) -> Option<Arc<RealtimeApplication>> {
    let application = Arc::new(application);

    self
      .applications
      .insert(application.id.clone(), application)
  }

  pub fn get(&self, application_id: &ApplicationId) -> Option<Arc<RealtimeApplication>> {
    self.applications.get(application_id).cloned()
  }
}

impl Default for ApplicationRegistry {
  fn default() -> Self {
    Self::new()
  }
}
