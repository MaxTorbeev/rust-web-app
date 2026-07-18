use redis::aio::MultiplexedConnection;
use redis::RedisResult;
use crate::RedisConfig;

pub struct RedisClient {
  connection: MultiplexedConnection,
}

impl RedisClient {
  pub async fn connect(config: &RedisConfig) ->  RedisResult<Self>  {
    let client = redis::Client::open(config.to_url());

    let connection = client?.get_multiplexed_async_connection().await?;

    Ok(Self { connection })
  }

  pub async fn ping(&self) -> RedisResult<String> {
    let mut conn = self.connection.clone();

    redis::cmd("PING")
      .query_async::<String>(&mut conn)
      .await
  }


  pub async fn set(&self, key: &str, value: &str) -> RedisResult<String> {
    let mut conn = self.connection.clone();

    redis::cmd("SET")
      .arg(key)
      .arg(value)
      .query_async::<String>(&mut conn)
      .await
  }
}