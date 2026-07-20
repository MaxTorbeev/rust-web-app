use serde::{Deserialize, Serialize};
use crate::{Connection, Message, PresenceMessage, ProtocolAction};

#[derive(Clone, Serialize, Deserialize)]
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
  pub fn connected(connection: &Connection) -> Self {
    Self {
      action: ProtocolAction::Connected,
      channel: None,
      messages: None,
      presence: None,
      msg_serial: None,
      connection_id: Some(connection.id.as_str().to_string()),
    }
  }

  pub fn attached(message: &ProtocolMessage) -> Self {
    Self {
      action: ProtocolAction::Attached,
      channel: message.channel.clone(),
      messages: None,
      presence: None,
      msg_serial: message.msg_serial,
      connection_id: None,
    }
  }

  pub fn ack(message: &ProtocolMessage) -> Self {
    Self {
      action: ProtocolAction::Ack,
      channel: message.channel.clone(),
      messages: None,
      presence: None,
      msg_serial: message.msg_serial,
      connection_id: None,
    }
  }

  pub fn heartbeat() -> Self {
    Self {
      action: ProtocolAction::Heartbeat,
      channel: None,
      messages: None,
      presence: None,
      msg_serial: None,
      connection_id: None,
    }
  }

  pub fn nack(msg_serial: Option<u64>) -> Self {
    Self {
      action: ProtocolAction::Nack,
      channel: None,
      messages: None,
      presence: None,
      msg_serial,
      connection_id: None,
    }
  }
}