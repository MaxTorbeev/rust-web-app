use crate::JetStreamPublisherError;

pub struct JetStreamPublisherConfig {
  pub subject_prefix: String
}

impl JetStreamPublisherConfig {
  pub fn try_from_env() -> Result<Self, JetStreamPublisherError> {
    let app_name = read_env("APP")?;
    let app_env = read_env("APP_ENV")?;

    Ok(Self {
      subject_prefix: format!("{}:{}:{}", app_name, app_env, "events")
    })
  }
}

fn read_env(variable: &'static str) -> Result<String, JetStreamPublisherError> {
  let value = std::env::var(variable)
    .map_err(|source| JetStreamPublisherError::MissingEnv {
      variable,
      source,
    })?;

  validate_env_value(variable, &value)?;

  Ok(value)
}

fn validate_env_value(variable: &'static str, value: &str) -> Result<(), JetStreamPublisherError> {
  if value.is_empty() {
    return Err(JetStreamPublisherError::InvalidEnv {
      variable,
      value: value.to_string(),
      reason: "value must not be empty".to_string(),
    });
  }

  if value.chars().any(|character| character.is_ascii_whitespace()) {
    return Err(JetStreamPublisherError::InvalidEnv {
      variable,
      value: value.to_string(),
      reason: "value must not contain whitespace".to_string(),
    });
  }

  if value.contains('*') || value.contains('>') {
    return Err(JetStreamPublisherError::InvalidEnv {
      variable,
      value: value.to_string(),
      reason: "value must not contain NATS wildcards '*' or '>'".to_string(),
    });
  }

  Ok(())
}
