pub enum SessionError {
  TokenGeneration(getrandom::Error),
  Redis(redis_client::RedisError),
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