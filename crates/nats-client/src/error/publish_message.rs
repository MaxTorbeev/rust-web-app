use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum PublishMessageError {
  #[error("JetStream publish subject must not be empty or contain whitespace")]
  InvalidSubject,

  #[error("JetStream message ID must not be empty")]
  EmptyMessageId,

  #[error("JetStream message ID must not contain CR or LF characters")]
  InvalidMessageId,
}
