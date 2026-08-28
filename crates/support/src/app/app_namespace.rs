use super::{AppNamespaceError, read_env};
use std::fmt::{Display, Formatter};

pub const APP_NAMESPACE_SEPARATOR: &str = ".";

/// Canonical namespace shared by application infrastructure adapters.
///
/// # Examples
///
/// ```
/// use support::app::AppNamespace;
///
/// # fn main() -> Result<(), support::app::AppNamespaceError> {
/// let namespace = AppNamespace::try_new(
///   "mxt_realtime",
///   "production",
///   "event-bus",
///   1,
/// )?;
///
/// assert_eq!(namespace.as_str(), "mxt_realtime.production.event-bus.v1");
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AppNamespace(String);

impl AppNamespace {
  /// Builds and validates a namespace from explicit values.
  pub fn try_new(
    app: impl Into<String>,
    app_environment: impl Into<String>,
    subsystem: impl Into<String>,
    version: u64,
  ) -> Result<Self, AppNamespaceError> {
    let app = app.into();
    let app_environment = app_environment.into();
    let subsystem = subsystem.into();

    validate_namespace_segment("APP", &app)?;
    validate_namespace_segment("APP_ENV", &app_environment)?;
    validate_namespace_segment("subsystem", &subsystem)?;

    if version == 0 {
      return Err(AppNamespaceError::InvalidVersion { version });
    }

    let version = format!("v{version}");

    Ok(Self(
      [app, app_environment, subsystem, version].join(APP_NAMESPACE_SEPARATOR),
    ))
  }

  /// Reads `APP` and `APP_ENV`, then builds and validates a namespace.
  pub fn try_from_env(
    subsystem: impl Into<String>,
    version: u64,
  ) -> Result<Self, AppNamespaceError> {
    Self::try_new(read_env("APP")?, read_env("APP_ENV")?, subsystem, version)
  }

  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl AsRef<str> for AppNamespace {
  fn as_ref(&self) -> &str {
    self.as_str()
  }
}

impl Display for AppNamespace {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
    formatter.write_str(self.as_str())
  }
}

fn validate_namespace_segment(field: &'static str, value: &str) -> Result<(), AppNamespaceError> {
  if value.is_empty() {
    return Err(AppNamespaceError::InvalidNamespaceSegment {
      field,
      value: value.to_owned(),
      reason: "value must not be empty",
    });
  }

  if !value
    .bytes()
    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
  {
    return Err(AppNamespaceError::InvalidNamespaceSegment {
      field,
      value: value.to_owned(),
      reason: "value may contain only ASCII letters, digits, '-' and '_'",
    });
  }

  Ok(())
}
