use crate::{ApplicationId, ApplicationKeyName, ConfigError};

pub struct RealtimeConfig {
  pub application_id: ApplicationId,
  pub key_name: String,
  pub key_secret: String,
}

impl RealtimeConfig {
  pub fn from_env() -> Result<Self, ConfigError> {
    let credentials = std::env::var("APP_REALTIME_API_KEY")
      .map_err(ConfigError::Environment)?;

    Self::parse(&credentials)
  }

  fn parse(credentials: &str) -> Result<Self, ConfigError> {
    let (key_name, key_secret) = credentials
      .split_once(':')
      .ok_or_else(|| ConfigError::InvalidApiKeyFormat)?;

    let application_key_name = key_name
      .parse::<ApplicationKeyName>()
      .map_err(|_| ConfigError::InvalidApiKeyFormat)?;

    if key_secret.is_empty() {
      return Err(ConfigError::InvalidApiKeyFormat);
    }

    Ok(Self {
      application_id: application_key_name
        .application_id()
        .clone(),
      key_name: key_name.to_string(),
      key_secret: key_secret.to_string(),
    })
  }
}