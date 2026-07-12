use redis::aio::MultiplexedConnection;

pub async fn ping(connection: &MultiplexedConnection) -> redis::RedisResult<String> {
  let mut conn = connection.clone();

  redis::cmd("PING")
    .query_async::<String>(&mut conn)
    .await
}