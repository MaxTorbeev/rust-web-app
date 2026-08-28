use std::env::VarError;
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("failed to read environment variable `{variable}`: {source}")]
pub struct ReadEnvError {
  variable: String,

  #[source]
  source: VarError,
}

impl ReadEnvError {
  pub(crate) fn new(variable: impl Into<String>, source: VarError) -> Self {
    Self {
      variable: variable.into(),
      source,
    }
  }

  pub fn variable(&self) -> &str {
    &self.variable
  }

  pub fn var_error(&self) -> &VarError {
    &self.source
  }
}
