#[derive(Debug)]
pub enum ConfigError {
  Environment(std::env::VarError),
  InvalidApiKeyFormat
}

impl std::fmt::Display for ConfigError {
  fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Environment(error) => {
        write!(fmt, "real time environment error: {}", error)
      },
      Self::InvalidApiKeyFormat => {
        write!(fmt, "invalid api key format")
      }
    }
  }
}

impl std::error::Error for ConfigError {}