use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{Event, EventMessageError};

/// Transport-independent representation of one domain event.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventMessage {
  event_id: Uuid,
  event_name: String,
  schema_version: u16,
  payload: Value,
}

impl EventMessage {
  /// Creates a message from already decoded transport fields.
  pub fn new(
    event_id: Uuid,
    event_name: impl Into<String>,
    schema_version: u16,
    payload: Value,
  ) -> Self {
    Self {
      event_id,
      event_name: event_name.into(),
      schema_version,
      payload,
    }
  }

  /// Creates a message with a new identifier.
  ///
  /// The returned value must be reused for all retries of the same publication.
  pub fn try_from_event<E>(event: &E) -> Result<Self, EventMessageError>
  where
    E: Event,
  {
    Self::try_from_event_with_id(Uuid::new_v4(), event)
  }

  /// Creates a message with an existing identifier.
  pub fn try_from_event_with_id<E>(event_id: Uuid, event: &E) -> Result<Self, EventMessageError>
  where
    E: Event,
  {
    let payload = serde_json::to_value(event).map_err(EventMessageError::Encode)?;

    Ok(Self::new(event_id, E::NAME, E::VERSION, payload))
  }

  pub fn event_id(&self) -> Uuid {
    self.event_id
  }

  pub fn event_name(&self) -> &str {
    &self.event_name
  }

  pub fn schema_version(&self) -> u16 {
    self.schema_version
  }

  pub fn payload(&self) -> &Value {
    &self.payload
  }

  /// Serializes the complete envelope for a transport such as NATS.
  pub fn to_bytes(&self) -> Result<Vec<u8>, EventMessageError> {
    serde_json::to_vec(self).map_err(EventMessageError::Encode)
  }

  /// Restores a complete envelope received from transport.
  pub fn from_bytes(bytes: &[u8]) -> Result<Self, EventMessageError> {
    serde_json::from_slice(bytes).map_err(EventMessageError::Decode)
  }

  /// Decodes the typed domain event stored in the payload.
  pub fn decode_event<E>(&self) -> Result<E, EventMessageError>
  where
    E: Event,
  {
    if self.event_name != E::NAME {
      return Err(EventMessageError::EventTypeMismatch {
        expected: E::NAME.to_string(),
        actual: self.event_name.clone(),
      });
    }

    if self.schema_version != E::VERSION {
      return Err(EventMessageError::EventVersionMismatch {
        event_name: self.event_name.clone(),
        expected: E::VERSION,
        actual: self.schema_version,
      });
    }

    serde_json::from_value(self.payload.clone()).map_err(EventMessageError::Decode)
  }
}
