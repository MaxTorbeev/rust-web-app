use thiserror::Error;

/// Validation error for [`crate::NatsConfig`].
///
/// Ошибка проверки конфигурации подключения [`crate::NatsConfig`].
#[derive(Debug, Error, Eq, PartialEq)]
#[non_exhaustive]
pub enum NatsConfigError {
  #[error("at least one NATS server is required")]
  NoServers,

  #[error("invalid NATS server at index {index}: {value:?}")]
  InvalidServer { index: usize, value: String },
}

/// Validation error for [`crate::ConsumerConfig`].
///
/// Ошибка проверки конфигурации JetStream consumer-а
/// [`crate::ConsumerConfig`].
#[derive(Debug, Error, Eq, PartialEq)]
#[non_exhaustive]
pub enum ConsumerConfigError {
  #[error("invalid JetStream stream name {value:?}")]
  InvalidStreamName { value: String },

  #[error("invalid JetStream durable consumer name {value:?}")]
  InvalidDurableName { value: String },

  #[error("invalid JetStream consumer filter subject {value:?}")]
  InvalidFilterSubject { value: String },

  #[error("JetStream consumer ack_wait must be greater than zero")]
  ZeroAckWait,

  #[error("JetStream consumer max_deliver must be greater than zero, got {max_deliver}")]
  InvalidMaxDeliver { max_deliver: i64 },
}

/// Validation error for [`crate::StreamLimits`].
///
/// Ошибка проверки ограничений хранения [`crate::StreamLimits`].
#[derive(Debug, Error, Eq, PartialEq)]
#[non_exhaustive]
pub enum StreamLimitsError {
  #[error("JetStream max_messages must be greater than zero, got {max_messages}")]
  InvalidMaxMessages { max_messages: i64 },

  #[error("JetStream max_message_size must be greater than zero, got {max_message_size}")]
  InvalidMaxMessageSize { max_message_size: i32 },

  #[error("JetStream max_bytes must be greater than zero, got {max_bytes}")]
  InvalidMaxBytes { max_bytes: i64 },

  #[error("JetStream max_age must be greater than zero")]
  ZeroMaxAge,
}

/// Validation error for [`crate::StreamConfig`].
///
/// Ошибка проверки конфигурации JetStream stream-а [`crate::StreamConfig`].
#[derive(Debug, Error, Eq, PartialEq)]
#[non_exhaustive]
pub enum StreamConfigError {
  #[error("invalid JetStream stream name {value:?}")]
  InvalidName { value: String },

  #[error("at least one JetStream stream subject is required")]
  NoSubjects,

  #[error("invalid JetStream stream subject {value:?}")]
  InvalidSubject { value: String },

  #[error("JetStream stream subjects must be unique")]
  DuplicateSubjects,

  #[error("JetStream replicas must be between 1 and 5, got {replicas}")]
  InvalidReplicas { replicas: usize },

  #[error("JetStream duplicate_window must be greater than zero")]
  ZeroDuplicateWindow,
}
