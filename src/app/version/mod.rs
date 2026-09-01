const DEVELOPMENT_REVISION: &str = "development";

/// Version and source revision embedded in the application binary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AppVersion {
  version: &'static str,
  revision: &'static str,
}

impl AppVersion {
  pub(crate) const CURRENT: Self = Self {
    version: env!("CARGO_PKG_VERSION"),
    revision: match option_env!("APP_BUILD_REVISION") {
      Some(revision) => revision,
      None => DEVELOPMENT_REVISION,
    },
  };

  pub(crate) const fn version(&self) -> &'static str {
    self.version
  }

  pub(crate) const fn revision(&self) -> &'static str {
    self.revision
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn current_contains_compile_time_version() {
    assert_eq!(AppVersion::CURRENT.version(), env!("CARGO_PKG_VERSION"));
  }

  #[test]
  fn current_contains_compile_time_revision() {
    let expected = option_env!("APP_BUILD_REVISION").unwrap_or(DEVELOPMENT_REVISION);

    assert_eq!(AppVersion::CURRENT.revision(), expected);
  }
}
