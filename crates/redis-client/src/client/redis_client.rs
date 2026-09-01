use crate::{RedisClientError, RedisClientResult, RedisConfig, ScriptValue};
use redis::aio::{ConnectionManager, ConnectionManagerConfig};

/// Управляет соединением с Redis и выполняет поддерживаемые команды.
///
/// Конкретный Redis-драйвер скрыт внутри структуры. Публичные методы возвращают
/// только типы, объявленные в `redis-client`.
pub struct RedisClient {
  connection: ConnectionManager,
}

impl RedisClient {
  pub async fn connect(config: &RedisConfig) -> RedisClientResult<Self> {
    let client = redis::Client::open(config.to_url()).map_err(RedisClientError::connection)?;

    let manager_config = ConnectionManagerConfig::new()
      .set_connection_timeout(config.connection_timeout)
      .set_response_timeout(config.response_timeout);

    let connection = client
      .get_connection_manager_with_config(manager_config)
      .await
      .map_err(RedisClientError::connection)?;

    Ok(Self { connection })
  }

  pub async fn ping(&self) -> RedisClientResult<()> {
    let mut conn = self.connection.clone();

    let response = redis::cmd("PING")
      .query_async::<String>(&mut conn)
      .await
      .map_err(RedisClientError::command)?;

    if response == "PONG" {
      Ok(())
    } else {
      Err(RedisClientError::unexpected_ping_response())
    }
  }

  pub async fn set(&self, key: &str, value: &str) -> RedisClientResult<String> {
    let mut conn = self.connection.clone();

    redis::cmd("SET")
      .arg(key)
      .arg(value)
      .query_async::<String>(&mut conn)
      .await
      .map_err(RedisClientError::command)
  }

  pub async fn get(&self, key: &str) -> RedisClientResult<String> {
    let mut conn = self.connection.clone();

    redis::cmd("GET")
      .arg(key)
      .query_async::<String>(&mut conn)
      .await
      .map_err(RedisClientError::command)
  }

  /// Выполняет Lua-скрипт через текущее управляемое Redis-соединение.
  ///
  /// Элементы `keys` передаются скрипту как `KEYS`, а элементы `args` — как
  /// `ARGV`. Байтовые срезы позволяют безопасно передавать не только строки,
  /// но и произвольные бинарные значения.
  ///
  /// Redis сначала пытается выполнить скрипт по SHA1 через `EVALSHA`. Если
  /// сервер ещё не знает этот скрипт, клиент загружает его и повторяет вызов.
  /// Ошибки соединения, таймауты и ошибки самого скрипта возвращаются как
  /// [`RedisClientError`](crate::RedisClientError).
  ///
  /// # Examples
  ///
  /// ```no_run
  /// use redis_client::{RedisClient, RedisClientResult, ScriptValue};
  ///
  /// async fn execute(redis: &RedisClient) -> RedisClientResult<()> {
  ///   const SCRIPT: &str = "return { KEYS[1], ARGV[1] }";
  ///
  ///   let key: &[u8] = b"event:42";
  ///   let token: &[u8] = b"lease-token";
  ///
  ///   let result = redis
  ///     .invoke_script(SCRIPT, &[key], &[token])
  ///     .await?;
  ///
  ///   assert_eq!(
  ///     result,
  ///     ScriptValue::Array(vec![
  ///       ScriptValue::Bytes(b"event:42".to_vec()),
  ///       ScriptValue::Bytes(b"lease-token".to_vec()),
  ///     ]),
  ///   );
  ///
  ///   Ok(())
  /// }
  /// ```
  pub async fn invoke_script(
    &self,
    source: &str,
    keys: &[&[u8]],
    args: &[&[u8]],
  ) -> RedisClientResult<ScriptValue> {
    let script = redis::Script::new(source);
    let mut invocation = script.prepare_invoke();

    for key in keys {
      invocation.key(*key);
    }

    for arg in args {
      invocation.arg(*arg);
    }

    let mut connection = self.connection.clone();

    let value: redis::Value = invocation
      .invoke_async(&mut connection)
      .await
      .map_err(RedisClientError::command)?;

    ScriptValue::from_driver(value)
  }
}
