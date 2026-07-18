use std::sync::Arc;
use crate::{SessionError, Token};
use redis_client::RedisClient;
use crate::session::session::Session;

pub struct SessionStore {
  redis: Arc<RedisClient>,
}

impl SessionStore {
  pub fn new(redis: Arc<RedisClient>) -> Self {
    Self { redis }
  }

  pub async fn create(&self, session: &Session) -> Result<String, SessionError> {
    let token = Token::generate()?;
    let key = Self::session_key(&token);

    let value = serde_json::to_string(&session)?;

    self.redis.set(&key, &value).await?;

    Ok(token)
  }

  pub async fn find(&self, token: &str) -> Result<String, SessionError> {
    let key = Self::session_key(&token);

    let token = self.redis.get(&key).await?;

    Ok(token)
  }

  fn session_key(token: &str) -> String {
    let fingerprint = Token::fingerprint(&token);

    format!("auth:session:{fingerprint}")
  }
}