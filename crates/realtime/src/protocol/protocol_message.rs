use serde::{Deserialize, Serialize};
use crate::{Connection, ConnectionDetails, Message, PresenceMessage, ProtocolAction, ProtocolFlag};

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

  #[serde(skip_serializing_if="Option::is_none")]
  pub flags: Option<u64>,
}

impl ProtocolMessage {
  fn with_action(action: ProtocolAction) -> Self {
    Self {
      action,
      channel: None,
      messages: None,
      presence: None,
      msg_serial: None,
      connection_id: None,
      connection_details: None,
      auth: None,
      count: None,
      flags: None,
    }
  }

  fn response_to(action: ProtocolAction, request: &ProtocolMessage) -> Self {
    Self {
      channel: request.channel.clone(),
      msg_serial: request.msg_serial,
      ..Self::with_action(action)
    }
  }

  fn for_channel(action: ProtocolAction, request: &Self) -> Self {
    Self {
      channel: request.channel.clone(),
      ..Self::with_action(action)
    }
  }

  pub fn connected(connection: &Connection) -> Self {
    Self {
      connection_id: Some(connection.id.as_str().to_string()),
      connection_details: Some(ConnectionDetails::from(connection)),
      ..Self::with_action(ProtocolAction::Connected)
    }
  }

  pub fn attached(message: &ProtocolMessage, flags: ProtocolFlag) -> Self {
    Self {
      flags: (!flags.is_empty()).then_some(flags.bits()),
      ..Self::for_channel(ProtocolAction::Attached, message)
    }
  }

  pub fn ack(request: &Self) -> Self {
    Self {
      msg_serial: request.msg_serial,
      count: request.msg_serial.map(|_| 1),
      ..Self::with_action(ProtocolAction::Ack)
    }
  }

  pub fn heartbeat() -> Self {
    Self::with_action(ProtocolAction::Heartbeat)
  }

  pub fn nack(msg_serial: Option<u64>) -> Self {
    Self {
      msg_serial,
      count: msg_serial.map(|_| 1),
      ..Self::with_action(ProtocolAction::Nack)
    }
  }

  pub fn presence(channel: &str, presence: Vec<PresenceMessage>) -> Self {
    Self {
      channel: Some(channel.to_string()),
      presence: Some(presence),
      ..Self::with_action(ProtocolAction::Presence)
    }
  }

  pub fn message(channel: &str, messages: Vec<Message>) -> Self {
    Self {
      channel: Some(channel.to_string()),
      messages: Some(messages),
      ..Self::with_action(ProtocolAction::Message)
    }
  }

  pub fn detached(message: &ProtocolMessage) -> Self {
    Self::for_channel(ProtocolAction::Detached, message)
  }

  pub fn disconnected() -> Self {
    Self::with_action(ProtocolAction::Disconnected)
  }

  pub fn sync(channel: &str, presence: Vec<PresenceMessage>) -> Self {
    Self {
      channel: Some(channel.to_string()),
      presence: Some(presence),
      ..Self::with_action(ProtocolAction::Sync)
    }
  }
}