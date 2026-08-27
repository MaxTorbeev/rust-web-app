use thiserror::Error;

/// Error returned when JetStream delivery metadata cannot be read or validated.
///
/// Ошибка чтения или проверки метаданных доставки JetStream.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct MessageMetadataError(MessageMetadataErrorSource);

#[derive(Debug, Error)]
enum MessageMetadataErrorSource {
    #[error("failed to read JetStream message metadata: {0}")]
    Parse(#[source] async_nats::Error),

    #[error("JetStream returned an invalid delivery attempt {0}")]
    InvalidAttempt(i64),
}

impl MessageMetadataError {
    pub(crate) fn parse(source: async_nats::Error) -> Self {
        Self(MessageMetadataErrorSource::Parse(source))
    }

    pub(crate) fn invalid_attempt(delivered: i64) -> Self {
        Self(MessageMetadataErrorSource::InvalidAttempt(delivered))
    }
}
