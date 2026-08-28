use support::app::ReadEnvError;

#[derive(Debug)]
pub enum ConfigError {
  Environment(ReadEnvError),
  InvalidApiKeyFormat,
}

impl std::fmt::Display for ConfigError {
  fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Environment(error) => {
        write!(fmt, "real time environment error: {}", error)
      }
      Self::InvalidApiKeyFormat => {
        write!(fmt, "invalid api key format")
      }
    }
  }
}

impl std::error::Error for ConfigError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      Self::Environment(error) => Some(error),
      Self::InvalidApiKeyFormat => None,
    }
  }
}
