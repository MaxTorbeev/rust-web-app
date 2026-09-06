use crate::{CommittedChannelTransition, PresenceRejection};
use serde::{Deserialize, Serialize};

/// Итог обработки команды изменения Presence.
///
/// Команда либо полностью фиксируется и возвращает описание произошедшего
/// перехода, либо отклоняется без изменения состояния. Частичное применение
/// элементов одной команды не допускается.
///
/// Инфраструктурные ошибки хранилища в этот тип не входят и возвращаются через
/// `Result<_, ChannelStateStoreError>`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PresenceMutationOutcome {
  /// Все изменения команды зафиксированы.
  Committed(CommittedChannelTransition),

  /// Команда отклонена без изменения Presence.
  Rejected(PresenceRejection),
}

/// Результат обработки команды изменения Presence.
///
/// Содержит доменный результат операции и указывает, была ли команда выполнена
/// впервые или хранилище вернуло результат ранее обработанной команды с тем же
/// ключом дедупликации.
///
/// Повторная обработка не изменяет Presence, не создаёт новую ревизию и событие.
/// Клиенту при этом должен быть возвращён тот же ACK или NACK, что и при первой
/// обработке команды.
#[derive(Clone, Debug)]
pub struct PresenceMutationReceipt {
  /// Зафиксированный или отклонённый результат операции.
  pub outcome: PresenceMutationOutcome,

  /// `true`, если результат загружен из журнала ранее обработанных операций;
  /// `false`, если команда была обработана впервые.
  pub replayed: bool,
}

impl PresenceMutationReceipt {
  /// Результат команды, обработанной впервые.
  pub fn fresh(outcome: PresenceMutationOutcome) -> Self {
    Self {
      outcome,
      replayed: false,
    }
  }

  /// Результат, загруженный из журнала ранее обработанных операций.
  pub fn replayed(outcome: PresenceMutationOutcome) -> Self {
    Self {
      outcome,
      replayed: true,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{CommittedPresenceEvent, PresenceChannelChanged};
  use serde_json::json;
  use uuid::Uuid;

  /// Формат результата операции — это формат журнала дедупликации и будущего
  /// Redis-хранилища: wire-соглашение `camelCase` для полей и вариантов.
  #[test]
  fn rejected_outcome_uses_camel_case_wire_format() {
    let outcome = PresenceMutationOutcome::Rejected(PresenceRejection::ClientIdNotAllowed {
      client_id: "client-1".to_owned(),
    });

    let encoded = serde_json::to_value(&outcome).unwrap();

    assert_eq!(
      encoded,
      json!({ "rejected": { "clientIdNotAllowed": { "clientId": "client-1" } } }),
    );

    let unit = serde_json::to_value(PresenceMutationOutcome::Rejected(PresenceRejection::NotAttached)).unwrap();
    assert_eq!(unit, json!({ "rejected": "notAttached" }));

    let decoded: PresenceMutationOutcome = serde_json::from_value(encoded).unwrap();
    assert!(matches!(
      decoded,
      PresenceMutationOutcome::Rejected(PresenceRejection::ClientIdNotAllowed { client_id }) if client_id == "client-1"
    ));
  }

  #[test]
  fn committed_outcome_uses_camel_case_wire_format() {
    let event_id = Uuid::parse_str("a15bb6d5-51ea-47db-a9a5-08b41b3b2d91").unwrap();
    let change: PresenceChannelChanged = serde_json::from_value(json!({
      "channel": { "applicationId": "application-1", "channel": "room-1" },
      "origin": {
        "nodeId": "node-1",
        "bootGeneration": "293a2951-5ba0-482c-91c7-0a0c72a5ce4b",
        "startedAt": 1_700_000_000_000_u64
      },
      "presenceRevision": 3,
      "occupancyVersion": 5,
      "memberChanges": [{
        "action": "leave",
        "connectionId": "connection-1",
        "clientId": "client-1",
        "data": null,
        "messageId": "connection-1:7:0",
        "timestamp": 1_700_000_000_000_u64
      }],
      "occupancy": {
        "metrics": {
          "connections": 1,
          "publishers": 1,
          "subscribers": 1,
          "presenceConnections": 1,
          "presenceSubscribers": 1,
          "presenceMembers": 0
        },
        "changedCategories": ["presenceMembers"],
        "zeroBoundaryCategories": ["presenceMembers"]
      },
      "occurredAt": 1_700_000_000_000_u64
    }))
    .expect("canonical change must deserialize from camelCase");

    let outcome = PresenceMutationOutcome::Committed(CommittedChannelTransition::Changed(
      CommittedPresenceEvent::new(event_id, change),
    ));

    let encoded = serde_json::to_value(&outcome).unwrap();
    let event = &encoded["committed"]["changed"];

    assert_eq!(event["eventId"], json!("a15bb6d5-51ea-47db-a9a5-08b41b3b2d91"));
    assert_eq!(event["change"]["memberChanges"][0]["messageId"], json!("connection-1:7:0"));
    assert_eq!(event["change"]["occupancy"]["changedCategories"], json!(["presenceMembers"]));

    // Ни одного snake_case ключа быть не должно.
    let text = encoded.to_string();
    assert!(!text.contains('_'), "unexpected snake_case key in {text}");
  }
}
