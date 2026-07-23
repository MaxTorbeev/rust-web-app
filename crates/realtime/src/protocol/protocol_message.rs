use serde::{Deserialize, Serialize};
use crate::{Connection, ConnectionDetails, Message, PresenceMessage, ProtocolAction};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthDetails {
  pub access_token: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

  #[serde(skip_serializing_if="Option::is_none")]
  pub connection_details: Option<ConnectionDetails>,

  #[serde(skip_serializing_if="Option::is_none")]
  pub auth: Option<AuthDetails>,

  #[serde(skip_serializing_if="Option::is_none")]
  pub count: Option<u64>,
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
      connection_details: Some(ConnectionDetails::new(connection)),
      auth: None,
      count: None,
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
      connection_details: None,
      auth: None,
      count: None,
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
      connection_details: None,
      auth: None,
      count: message.msg_serial.map(|_| 1),
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
      connection_details: None,
      auth: None,
      count: None,
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
      connection_details: None,
      auth: None,
      count: msg_serial.map(|_| 1),
    }
  }

  pub fn presence(channel: &str, presence: Vec<PresenceMessage>) -> Self {
    Self {
      action: ProtocolAction::Presence,
      channel: Some(channel.to_string()),
      messages: None,
      presence: Some(presence),
      msg_serial: None,
      connection_id: None,
      connection_details: None,
      auth: None,
      count: None,
    }
  }
}