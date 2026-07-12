use redis::aio::MultiplexedConnection;
use redis::RedisResult;
use crate::config::RedisConfig;

mod ping;

pub async fn connect(config: &RedisConfig) -> RedisResult<MultiplexedConnection> {
  let client = redis::Client::open(config.to_url());

  client?.get_multiplexed_async_connection().await
}

pub use ping::ping;