use crate::CommittedPresenceEvent;
use serde::{Deserialize, Serialize};

/// Результат успешно завершённой операции над состоянием канала.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum CommittedChannelTransition {
  /// Операция не создала нового события; версия Occupancy осталась прежней.
  Unchanged { occupancy_version: u64 },

  /// Изменение зафиксировано; обе версии хранятся в самом событии.
  Changed(CommittedPresenceEvent),
}

impl CommittedChannelTransition {
  /// Возвращает новую ревизию Presence, если операция изменила участников.
  pub fn presence_revision(&self) -> Option<u64> {
    match self {
      Self::Unchanged { .. } => None,
      Self::Changed(event) => event.change().presence_revision,
    }
  }

  /// Возвращает версию Occupancy после операции.
  pub fn occupancy_version(&self) -> u64 {
    match self {
      Self::Unchanged { occupancy_version } => *occupancy_version,
      Self::Changed(event) => event.change().occupancy_version,
    }
  }

  /// Возвращает событие, созданное этой операцией.
  pub fn event(&self) -> Option<&CommittedPresenceEvent> {
    match self {
      Self::Unchanged { .. } => None,
      Self::Changed(event) => Some(event),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{PresenceChannelChanged, PresenceMutationOutcome};
  use serde_json::json;
  use uuid::Uuid;

  #[test]
  fn unchanged_round_trip_preserves_version_without_an_event() {
    let transition = CommittedChannelTransition::Unchanged {
      occupancy_version: 7,
    };

    let encoded = serde_json::to_value(&transition).unwrap();
    assert_eq!(encoded, json!({ "unchanged": { "occupancyVersion": 7 } }));

    let decoded: CommittedChannelTransition = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded.occupancy_version(), 7);
    assert_eq!(decoded.presence_revision(), None);
    assert!(decoded.event().is_none());
  }

  #[test]
  fn committed_outcome_round_trip_preserves_event_and_its_versions() {
    let event_id = Uuid::parse_str("a15bb6d5-51ea-47db-a9a5-08b41b3b2d91").unwrap();
    let change: PresenceChannelChanged = serde_json::from_value(json!({
      "channel": {
        "applicationId": "application-1",
        "channel": "room-1"
      },
      "origin": {
        "nodeId": "node-1",
        "bootGeneration": "293a2951-5ba0-482c-91c7-0a0c72a5ce4b",
        "startedAt": 1_700_000_000_000_u64
      },
      "presenceRevision": 11,
      "occupancyVersion": 7,
      "memberChanges": [],
      "occupancy": null,
      "occurredAt": 1_700_000_000_000_u64
    }))
    .unwrap();

    // Occupancy-only events have no new Presence revision.
    for presence_revision in [None, Some(11)] {
      let mut change = change.clone();
      change.presence_revision = presence_revision;
      let event = CommittedPresenceEvent::new(event_id, change);
      let outcome = PresenceMutationOutcome::Committed(CommittedChannelTransition::Changed(event));

      let encoded = serde_json::to_value(&outcome).unwrap();
      assert!(encoded["committed"].get("presenceRevision").is_none());
      assert!(encoded["committed"].get("occupancyVersion").is_none());

      let decoded: PresenceMutationOutcome = serde_json::from_value(encoded).unwrap();
      let PresenceMutationOutcome::Committed(transition) = decoded else {
        panic!("committed outcome must survive serialization");
      };

      assert_eq!(transition.presence_revision(), presence_revision);
      assert_eq!(transition.occupancy_version(), 7);
      assert_eq!(transition.event().unwrap().event_id(), event_id);
    }
  }
}
