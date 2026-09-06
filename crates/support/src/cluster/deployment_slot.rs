use crate::app::{ReadEnvError, read_env_or};
use serde::Serialize;
use std::fmt;

/// Принадлежность ноды к deployment-группе.
///
/// Slot не означает, что группа сейчас active: источник истины о маршрутизации
/// находится в load balancer и deployment state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DeploymentSlot {
  /// Текущий single-host deployment.
  Single,
  Blue,
  Green,
}

#[derive(Debug)]
pub enum DeploymentSlotError {
  Environment(ReadEnvError),
  /// Значение вне `single`, `blue`, `green`.
  Invalid(String),
}

impl fmt::Display for DeploymentSlotError {
  fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Environment(error) => write!(fmt, "deployment slot environment error: {error}"),
      Self::Invalid(value) => write!(
        fmt,
        "invalid DEPLOYMENT_SLOT `{value}`: expected single, blue or green"
      ),
    }
  }
}

impl std::error::Error for DeploymentSlotError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      Self::Environment(error) => Some(error),
      Self::Invalid(_) => None,
    }
  }
}

impl DeploymentSlot {
  /// Читает `DEPLOYMENT_SLOT`; отсутствующая переменная означает `single`.
  ///
  /// В настоящем blue-green deployment slot указывается явно.
  pub fn from_env() -> Result<Self, DeploymentSlotError> {
    let value =
      read_env_or("DEPLOYMENT_SLOT", "single").map_err(DeploymentSlotError::Environment)?;

    Self::parse(&value)
  }

  pub fn parse(value: &str) -> Result<Self, DeploymentSlotError> {
    match value {
      "single" => Ok(Self::Single),
      "blue" => Ok(Self::Blue),
      "green" => Ok(Self::Green),
      other => Err(DeploymentSlotError::Invalid(other.to_owned())),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_allowed_slots() {
    assert_eq!(
      DeploymentSlot::parse("single").unwrap(),
      DeploymentSlot::Single
    );
    assert_eq!(DeploymentSlot::parse("blue").unwrap(), DeploymentSlot::Blue);
    assert_eq!(
      DeploymentSlot::parse("green").unwrap(),
      DeploymentSlot::Green
    );
  }

  #[test]
  fn rejects_unknown_slot() {
    assert!(matches!(
      DeploymentSlot::parse("Blue"),
      Err(DeploymentSlotError::Invalid(value)) if value == "Blue"
    ));
    assert!(DeploymentSlot::parse("").is_err());
  }
}
