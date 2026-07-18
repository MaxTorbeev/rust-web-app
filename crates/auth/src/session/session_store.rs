use std::sync::Arc;
use crate::{SessionError, Token};
use redis_client::RedisClient;

pub struct SessionStore {
  redis: Arc<RedisClient>,
}

impl SessionStore {
  pub fn new(redis: Arc<RedisClient>) -> Self {
    Self { redis }
  }

  pub async fn create(&self, login: &str) -> Result<String, SessionError> {

    let token = Token::generate()?;
    let fingerprint = Token::fingerprint(&token);

    let key = format!("auth:session:{fingerprint}");

    self.redis.set(&key, login).await?;

    Ok(token)
  }
  //
  // pub async fn find(&self, token: &str) -> Result<Option<Session>, SessionError> {
  //
  // }
}