use std::env::VarError;
use std::time::Duration;
use support::app::{ReadEnvError, read_env, read_env_or};

pub struct RedisConfig {
  pub host: String,
  pub port: String,
  pub username: Option<String>,
  pub password: Option<String>,
  pub db: String,
  /// Сколько ждём установления TCP-соединения;
  pub connection_timeout: Duration,
  /// Сколько ждём ответа на Redis-команду.
  pub response_timeout: Duration,
}

impl Default for RedisConfig {
  fn default() -> Self {
    Self {
      connection_timeout: Duration::from_secs(5),
      response_timeout: Duration::from_secs(3),
      username: None,
      password: None,
      host: "127.0.0.1".to_owned(),
      port: "6379".to_owned(),
      db: "0".to_owned(),
    }
  }
}

impl RedisConfig {
  /// Builds a Redis configuration from environment variables.
  ///
  /// `REDIS_HOST` and `REDIS_PORT` retain their local defaults when absent.
  /// `REDIS_USERNAME` and `REDIS_PASSWORD` are optional. An environment value
  /// that is present but is not valid Unicode is returned as an error for every
  /// field.
  pub fn from_env() -> Result<Self, ReadEnvError> {
    let defaults = Self::default();

    Ok(Self {
      host: read_env_or("REDIS_HOST", defaults.host)?,
      port: read_env_or("REDIS_PORT", defaults.port)?,
      username: read_optional_env("REDIS_USERNAME")?,
      password: read_optional_env("REDIS_PASSWORD")?,
      ..defaults
    })
  }

  pub fn to_url(&self) -> String {
    let auth = match (self.username.as_ref(), self.password.as_ref()) {
      (Some(username), Some(password)) => format!("{username}:{password}"),
      (None, Some(password)) => format!(":{password}"),
      _ => String::new(),
    };

    format!("redis://{}@{}:{}/{}", auth, self.host, self.port, self.db)
  }
}

fn read_optional_env(variable: &'static str) -> Result<Option<String>, ReadEnvError> {
  match read_env(variable) {
    Ok(value) => Ok(Some(value)),
    Err(error) if matches!(error.var_error(), VarError::NotPresent) => Ok(None),
    Err(error) => Err(error),
  }
}

#[cfg(test)]
mod tests;
