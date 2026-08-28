use crate::{RedisClientError, RedisClientResult};

/// Значение, которое Lua-скрипт вернул через Redis.
///
/// Тип намеренно содержит только значения, необходимые прикладным адаптерам:
/// `nil`, целое число, бинарную строку и массив. Благодаря этому публичный API
/// `redis-client` не раскрывает типы внутреннего Redis-драйвера.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ScriptValue {
  /// Lua `nil`.
  Null,
  /// Целое число.
  Integer(i64),
  /// Строка или произвольное бинарное значение.
  Bytes(Vec<u8>),
  /// Последовательность вложенных значений.
  Array(Vec<ScriptValue>),
}

impl ScriptValue {
  pub(crate) fn from_driver(value: redis::Value) -> RedisClientResult<Self> {
    match value {
      redis::Value::Nil => Ok(Self::Null),
      redis::Value::Int(value) => Ok(Self::Integer(value)),
      redis::Value::BulkString(value) => Ok(Self::Bytes(value)),
      redis::Value::Array(values) => values
        .into_iter()
        .map(Self::from_driver)
        .collect::<RedisClientResult<Vec<_>>>()
        .map(Self::Array),
      redis::Value::SimpleString(value) => Ok(Self::Bytes(value.into_bytes())),
      redis::Value::Okay => Ok(Self::Bytes(b"OK".to_vec())),
      redis::Value::Map(_) => Err(RedisClientError::unsupported_script_value("map")),
      redis::Value::Attribute { .. } => {
        Err(RedisClientError::unsupported_script_value("attribute"))
      }
      redis::Value::Set(_) => Err(RedisClientError::unsupported_script_value("set")),
      redis::Value::Double(_) => Err(RedisClientError::unsupported_script_value("double")),
      redis::Value::Boolean(_) => Err(RedisClientError::unsupported_script_value("boolean")),
      redis::Value::VerbatimString { .. } => Err(RedisClientError::unsupported_script_value(
        "verbatim string",
      )),
      redis::Value::BigNumber(_) => Err(RedisClientError::unsupported_script_value("big number")),
      redis::Value::Push { .. } => Err(RedisClientError::unsupported_script_value("push")),
      redis::Value::ServerError(_) => {
        Err(RedisClientError::unsupported_script_value("server error"))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::RedisClientErrorKind;

  #[test]
  fn converts_supported_driver_values() {
    let value = redis::Value::Array(vec![
      redis::Value::Int(1),
      redis::Value::BulkString(b"lease-token".to_vec()),
      redis::Value::Nil,
    ]);

    assert_eq!(
      ScriptValue::from_driver(value).unwrap(),
      ScriptValue::Array(vec![
        ScriptValue::Integer(1),
        ScriptValue::Bytes(b"lease-token".to_vec()),
        ScriptValue::Null,
      ]),
    );
  }

  #[test]
  fn rejects_unsupported_driver_values() {
    let error = ScriptValue::from_driver(redis::Value::Boolean(true)).unwrap_err();

    assert_eq!(error.kind(), RedisClientErrorKind::Response);
  }
}
