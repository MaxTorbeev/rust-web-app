use crate::error::JetStreamPublisherConfigError;
use crate::subject::is_valid_subject_token;

/// Configuration required to map events to JetStream subjects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JetStreamPublisherConfig {
    subject_prefix: String,
}

impl JetStreamPublisherConfig {
    /// Builds a configuration from explicit application namespace components.
    pub fn try_new(
        app_name: impl Into<String>,
        app_environment: impl Into<String>,
    ) -> Result<Self, JetStreamPublisherConfigError> {
        let app_name = app_name.into();
        let app_environment = app_environment.into();

        validate_namespace_component("APP", &app_name)?;
        validate_namespace_component("APP_ENV", &app_environment)?;

        Ok(Self {
            subject_prefix: format!("{app_name}.{app_environment}.events"),
        })
    }

    /// Reads `APP` and `APP_ENV` and validates them before startup continues.
    pub fn try_from_env() -> Result<Self, JetStreamPublisherConfigError> {
        Self::try_new(read_env("APP")?, read_env("APP_ENV")?)
    }

    pub(crate) fn subject_prefix(&self) -> &str {
        &self.subject_prefix
    }
}

fn read_env(variable: &'static str) -> Result<String, JetStreamPublisherConfigError> {
    std::env::var(variable).map_err(|source| {
        JetStreamPublisherConfigError::MissingEnvironmentVariable { variable, source }
    })
}

fn validate_namespace_component(
    component: &'static str,
    value: &str,
) -> Result<(), JetStreamPublisherConfigError> {
    if value.is_empty() {
        return Err(JetStreamPublisherConfigError::InvalidNamespaceComponent {
            component,
            value: value.to_owned(),
            reason: "value must not be empty",
        });
    }

    if !is_valid_subject_token(value) {
        return Err(JetStreamPublisherConfigError::InvalidNamespaceComponent {
            component,
            value: value.to_owned(),
            reason: "value may contain only ASCII letters, digits, '-' and '_'",
        });
    }

    Ok(())
}
