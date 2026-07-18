use redis::aio::MultiplexedConnection;
use redis::RedisResult;


pub async fn del(connection: &MultiplexedConnection, key: &str) -> RedisResult<String> {
  let mut conn = connection.clone();

  redis::cmd("DEL")
    .query_async::<String>(&mut conn)
    .await
}