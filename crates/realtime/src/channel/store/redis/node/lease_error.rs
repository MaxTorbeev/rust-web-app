use redis_client::RedisClientError;
use redis_lease::RedisLeaseError;
use thiserror::Error;

/// Ошибка подготовки или выполнения операции node lease.
#[derive(Debug, Error)]
pub enum NodeLeaseError {
  #[error("node lease key does not match the Redis namespace")]
  NamespaceMismatch,

  #[error(transparent)]
  Redis(#[from] RedisClientError),

  #[error(transparent)]
  Lease(#[from] RedisLeaseError),

  #[error("failed to serialize node generation metadata: {0}")]
  Metadata(#[from] serde_json::Error),
}
