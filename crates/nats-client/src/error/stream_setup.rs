use async_nats::jetstream::context::CreateStreamError as DriverStreamError;
use thiserror::Error;

/// Error returned while creating, opening, or validating a JetStream stream.
///
/// Ошибка создания, открытия или проверки конфигурации JetStream stream-а.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct StreamSetupError(StreamSetupErrorSource);

#[derive(Debug, Error)]
enum StreamSetupErrorSource {
    #[error("failed to get or create JetStream stream: {0}")]
    Driver(#[source] DriverStreamError),

    #[error("JetStream stream `{stream_name}` has incompatible fields: {fields:?}")]
    IncompatibleConfiguration {
        stream_name: String,
        fields: Vec<&'static str>,
    },
}

impl StreamSetupError {
    /// Returns `true` when an existing stream differs from the requested
    /// configuration.
    ///
    /// Возвращает `true`, если конфигурация существующего stream-а отличается
    /// от запрошенной.
    pub fn is_incompatible_configuration(&self) -> bool {
        matches!(
            self.0,
            StreamSetupErrorSource::IncompatibleConfiguration { .. }
        )
    }

    pub(crate) fn from_driver(source: DriverStreamError) -> Self {
        Self(StreamSetupErrorSource::Driver(source))
    }

    pub(crate) fn incompatible_configuration(
        stream_name: impl Into<String>,
        fields: Vec<&'static str>,
    ) -> Self {
        Self(StreamSetupErrorSource::IncompatibleConfiguration {
            stream_name: stream_name.into(),
            fields,
        })
    }
}
