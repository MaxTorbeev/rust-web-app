use crate::{PresenceAction, PresenceChangeAction, PresenceMember, PresenceMemberChange};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresenceMessage {
  pub action: PresenceAction,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub client_id: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub connection_id: Option<String>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub timestamp: Option<u64>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub data: Option<serde_json::Value>,
}

impl From<&PresenceMember> for PresenceMessage {
  /// Участник из снимка канала для `SYNC`.
  fn from(member: &PresenceMember) -> Self {
    Self {
      action: PresenceAction::Present,
      id: Some(member.last_message_id.clone()),
      client_id: Some(member.client_id.clone()),
      connection_id: Some(member.connection_id.as_str().to_owned()),
      timestamp: Some(member.updated_at_ms),
      data: member.data.clone(),
    }
  }
}

impl From<&PresenceMemberChange> for PresenceMessage {
  /// Canonical delta зафиксированного события для `PRESENCE`.
  fn from(change: &PresenceMemberChange) -> Self {
    Self {
      action: change.action.into(),
      id: Some(change.message_id.clone()),
      client_id: Some(change.client_id.clone()),
      connection_id: Some(change.connection_id.as_str().to_owned()),
      timestamp: Some(change.timestamp.as_millis()),
      data: change.data.clone(),
    }
  }
}

impl From<PresenceChangeAction> for PresenceAction {
  fn from(action: PresenceChangeAction) -> Self {
    match action {
      PresenceChangeAction::Enter => Self::Enter,
      PresenceChangeAction::Update => Self::Update,
      PresenceChangeAction::Leave => Self::Leave,
    }
  }
}
