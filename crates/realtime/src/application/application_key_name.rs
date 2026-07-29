use crate::ApplicationId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidApplicationKeyName;

pub struct ApplicationKeyName {
  application_id: ApplicationId,
  key_id: String
}

impl ApplicationKeyName {
  pub fn application_id(&self) -> &ApplicationId {
    &self.application_id
  }
}

impl std::fmt::Display for InvalidApplicationKeyName {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(formatter, "application key name must have format <application_id>.<key_id>")
  }
}

impl std::error::Error for InvalidApplicationKeyName {}

impl std::str::FromStr for ApplicationKeyName {
  type Err = InvalidApplicationKeyName;

  fn from_str(value: &str) -> Result<Self, Self::Err> {
    let (application_id, key_id) = value
      .split_once('.')
      .ok_or(InvalidApplicationKeyName)?;

    if application_id.is_empty()
      || key_id.is_empty()
      || key_id.contains('.')
    {
      return Err(InvalidApplicationKeyName);
    }

    Ok(Self {
      application_id: ApplicationId::new(application_id),
      key_id: key_id.to_string(),
    })
  }
}