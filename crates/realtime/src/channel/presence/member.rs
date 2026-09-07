use crate::ConnectionId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use support::{NodeInstance, timestamp::Timestamp};

/// Участник Presence: одно `client_id` одного соединения в одном канале.
///
/// Сериализуемый формат — формат хранения участника в Redis; эталон
/// зафиксирован snapshot-тестом.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresenceMember {
  pub connection_id: ConnectionId,
  pub client_id: String,
  pub node_instance: NodeInstance,
  pub data: Option<Value>,
  /// `message_id` последней операции, изменившей участника; уходит клиенту как
  /// `id` записи `SYNC`.
  pub last_message_id: String,
  /// Ревизия Presence, на которой участник получил текущее состояние.
  pub presence_revision: u64,
  /// Время последней операции, изменившей участника.
  pub updated_at: Timestamp,
}

#[cfg(test)]
mod tests {
  use super::*;
  use support::{BootGeneration, NodeId};
  use uuid::Uuid;

  #[test]
  fn member_wire_format() {
    let member = PresenceMember {
      connection_id: serde_json::from_value(serde_json::json!("connection-1")).unwrap(),
      client_id: "client-1".to_owned(),
      node_instance: NodeInstance::new(
        NodeId::try_new("node-1").unwrap(),
        BootGeneration::from_uuid(Uuid::parse_str("293a2951-5ba0-482c-91c7-0a0c72a5ce4b").unwrap()),
        Timestamp::from_millis(1_700_000_000_000),
      ),
      data: Some(serde_json::json!({ "status": "online" })),
      last_message_id: "connection-1:7:0".to_owned(),
      presence_revision: 3,
      updated_at: Timestamp::from_millis(1_700_000_000_005),
    };

    insta::assert_json_snapshot!("presence_member", member);

    let decoded: PresenceMember =
      serde_json::from_value(serde_json::to_value(&member).unwrap()).unwrap();
    assert_eq!(decoded.updated_at, member.updated_at);
    assert_eq!(decoded.data, member.data);
  }
}
