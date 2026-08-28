use super::ReadEnvError;
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[non_exhaustive]
pub enum AppNamespaceError {
  #[error("{0}")]
  ReadEnvironmentVariable(#[from] ReadEnvError),

  #[error("invalid value {value:?} for namespace segment `{field}`: {reason}")]
  InvalidNamespaceSegment {
    field: &'static str,
    value: String,
    reason: &'static str,
  },

  #[error("invalid namespace version {version}: version must be greater than zero")]
  InvalidVersion { version: u64 },
}
