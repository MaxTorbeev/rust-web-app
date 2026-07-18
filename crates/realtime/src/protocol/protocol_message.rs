use serde::{Deserialize, Serialize};
use crate::{Message, PresenceMessage, ProtocolAction};

#[derive(Serialize, Deserialize)]
pub struct ProtocolMessage {

  pub action: ProtocolAction,

  #[serde(skip_serializing_if="Option::is_none")]
  pub channel: Option<String>,

  #[serde(skip_serializing_if="Option::is_none")]
  pub messages: Option<Vec<Message>>,

  #[serde(skip_serializing_if="Option::is_none")]
  pub presence: Option<Vec<PresenceMessage>>,

  #[serde(skip_serializing_if="Option::is_none")]
  pub msg_serial: Option<u64>,

  #[serde(skip_serializing_if="Option::is_none")]
  pub connection_id: Option<String>,
}

impl ProtocolMessage {
  pub fn connected() -> Self {
    Self {
      action: ProtocolAction::Connect,
      channel: None,
      messages: None,
      presence: None,
      msg_serial: None,
      connection_id: None,
    }
  }
}