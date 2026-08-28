use super::ReadEnvError;
use std::env::VarError;

/// Reads a Unicode environment variable while retaining its name on failure.
///
/// # Examples
///
/// ```no_run
/// use support::app::read_env;
///
/// # fn main() -> Result<(), support::app::ReadEnvError> {
/// let app = read_env("APP")?;
/// println!("{app}");
/// # Ok(())
/// # }
/// ```
pub fn read_env(variable: impl Into<String>) -> Result<String, ReadEnvError> {
  let variable = variable.into();

  std::env::var(&variable).map_err(|source| ReadEnvError::new(variable, source))
}

/// Reads a Unicode environment variable or returns `default` when it is absent.
///
/// An existing empty value is returned unchanged. A value that is not valid
/// Unicode remains an error.
///
/// # Examples
///
/// ```no_run
/// use support::app::read_env_or;
///
/// # fn main() -> Result<(), support::app::ReadEnvError> {
/// let environment = read_env_or("APP_ENV", "development")?;
/// println!("{environment}");
/// # Ok(())
/// # }
/// ```
pub fn read_env_or(
  variable: impl Into<String>,
  default: impl Into<String>,
) -> Result<String, ReadEnvError> {
  env_value_or(read_env(variable), default)
}

pub(super) fn env_value_or(
  value: Result<String, ReadEnvError>,
  default: impl Into<String>,
) -> Result<String, ReadEnvError> {
  match value {
    Err(error) if matches!(error.var_error(), VarError::NotPresent) => Ok(default.into()),
    value => value,
  }
}
