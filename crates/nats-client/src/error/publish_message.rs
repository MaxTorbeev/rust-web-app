use thiserror::Error;

/// Validation error for [`crate::PublishMessage`].
///
/// Ошибка проверки исходящего сообщения [`crate::PublishMessage`] до его
/// отправки в NATS.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum PublishMessageError {
  #[error("invalid JetStream publish subject")]
  InvalidSubject,

  #[error("JetStream message ID must not be empty")]
  EmptyMessageId,

  #[error("JetStream message ID must not contain CR or LF characters")]
  InvalidMessageId,
}
