#[derive(Debug)]
pub enum SessionError {
  TokenGeneration(getrandom::Error),
  Redis(redis_client::RedisError),
  Serialization(serde_json::Error),
}

impl From<getrandom::Error> for SessionError {
  fn from(err: getrandom::Error) -> SessionError {
    SessionError::TokenGeneration(err)
  }
}

impl From<redis_client::RedisError> for SessionError {
  fn from(err: redis_client::RedisError) -> SessionError {
    SessionError::Redis(err)
  }
}

impl From<serde_json::Error> for SessionError {
  fn from(err: serde_json::Error) -> SessionError {
    SessionError::Serialization(err)
  }
}
