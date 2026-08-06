use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use validator::{Validate, ValidationError};

/// https://ably.com/docs/api/rest-api#channel
#[derive(Debug, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BroadcastMessage {
  pub name: Option<String>,

  #[serde(default)]
  #[validate(custom(function = "validate_message_data"))]
  pub data: Value,

  #[validate(custom(function = "validate_encoding"))]
  pub encoding: Option<String>,

  #[validate(length(
    min = 1,
    code = "empty_client_id",
    message = "Client id must not be empty"
  ))]
  pub client_id: Option<String>,

  #[validate(length(
    min = 1,
    code = "empty_connection_key",
    message = "Connection key must not be empty"
  ))]
  pub connection_key: Option<String>,

  #[validate(length(
    min = 1,
    code = "empty_message_id",
    message = "Id must not be empty"
  ))]
  pub id: Option<String>,

  pub extras: Option<Map<String, Value>>,
}

fn validate_message_data(data: &Value) -> Result<(), ValidationError> {
  match data {
    Value::Null
    | Value::String(_)
    | Value::Array(_)
    | Value::Object(_) => Ok(()),

    Value::Bool(_)
    | Value::Number(_) => {
      let mut error =
        ValidationError::new("unsupported_data_type");

      error.message = Some(
        "message data must be a string, array, object, or null"
          .into()
      );

      Err(error)
    }
  }
}

fn validate_encoding(encoding: &str) -> Result<(), ValidationError> {
  if encoding.is_empty()
    || encoding.split('/').all(is_valid_encoding_segment)
  {
    return Ok(());
  }

  let mut error = ValidationError::new("invalid_encoding");

  error.message = Some(
    "encoding contains an invalid transformation chain".into()
  );

  Err(error)
}

fn is_valid_encoding_segment(segment: &str) -> bool {
  let (transform, parameter) = match segment.split_once('+') {
    Some((transform, parameter)) => {
      (transform, Some(parameter))
    }
    None => (segment, None),
  };

  is_valid_encoding_token(transform)
    && match parameter {
    Some(parameter) => is_valid_encoding_token(parameter),
    None => true,
  }
}

fn is_valid_encoding_token(token: &str) -> bool {
  !token.is_empty()
    && token.bytes().all(|byte| {
    byte.is_ascii_alphanumeric()
      || matches!(byte, b'-' | b'_')
  })
}