use crate::session::session::Session;
use crate::{SessionError, Token};
use redis_client::RedisClient;
use std::sync::Arc;

pub struct SessionStore {
  redis: Arc<RedisClient>,
}

impl SessionStore {
  pub fn new(redis: Arc<RedisClient>) -> Self {
    Self { redis }
  }

  pub async fn create(&self, session: Session) -> Result<String, SessionError> {
    let token = Token::generate()?;
    let key = Self::session_key(&token);
    let value = serde_json::to_string(&session)?;

    self.redis.set(&key, &value).await?;

    Ok(token)
  }

  pub async fn find(&self, token: &str) -> Result<Session, SessionError> {
    let key = Self::session_key(&token);

    let value = self.redis.get(&key).await?;

    let session = serde_json::from_str::<Session>(&value)?;

    Ok(session)
  }

  fn session_key(token: &str) -> String {
    let fingerprint = Token::fingerprint(&token);

    format!("auth:session:{fingerprint}")
  }
}
