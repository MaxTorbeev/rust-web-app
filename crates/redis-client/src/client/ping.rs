use redis::aio::MultiplexedConnection;
use redis::RedisResult;


pub async fn _del(connection: &MultiplexedConnection, _key: &str) -> RedisResult<String> {
  let mut conn = connection.clone();

  redis::cmd("DEL")
    .query_async::<String>(&mut conn)
    .await
}