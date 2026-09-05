use crate::{ChannelKey, ConnectionId, OccupancyChange};
use event_bus::{DeliveryClass, Event, EventMessage, EventMessageError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use support::{NodeInstance, timestamp::Timestamp};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceChangeAction {
  Enter,
  Update,
  Leave,
}

/// Полностью сформированное неизменяемое событие,
/// соответствующее уже зафиксированному изменению Presence.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommittedPresenceEvent {
  event_id: Uuid,
  change: PresenceChannelChanged,
}

impl CommittedPresenceEvent {
  pub fn new(event_id: Uuid, change: PresenceChannelChanged) -> Self {
    Self { event_id, change }
  }

  pub fn event_id(&self) -> Uuid {
    self.event_id
  }

  pub fn change(&self) -> &PresenceChannelChanged {
    &self.change
  }

  pub fn into_parts(self) -> (Uuid, PresenceChannelChanged) {
    (self.event_id, self.change)
  }
}

impl TryFrom<&CommittedPresenceEvent> for EventMessage {
  type Error = EventMessageError;

  fn try_from(event: &CommittedPresenceEvent) -> Result<Self, Self::Error> {
    Self::try_from_event_with_id(event.event_id, &event.change)
  }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PresenceChannelChanged {
  pub channel: ChannelKey,
  pub origin: NodeInstance,

  pub presence_revision: Option<u64>,
  pub occupancy_version: u64,

  pub member_changes: Vec<PresenceMemberChange>,
  pub occupancy: Option<OccupancyChange>,

  pub occurred_at: Timestamp,
}

impl Event for PresenceChannelChanged {
  const NAME: &'static str = "realtime.presence_channel_changed";
  const VERSION: u16 = 1;
  const DELIVERY: DeliveryClass = DeliveryClass::AllNodes;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PresenceMemberChange {
  pub action: PresenceChangeAction,
  pub connection_id: ConnectionId,
  pub client_id: String,
  pub data: Option<Value>,
  pub message_id: String,
  pub timestamp: Timestamp,
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn committed_event_preserves_its_id_in_event_message() {
    let event_id = Uuid::parse_str("a15bb6d5-51ea-47db-a9a5-08b41b3b2d91").unwrap();
    let change = serde_json::from_value::<PresenceChannelChanged>(json!({
      "channel": {
        "application_id": "application-1",
        "channel": "room-1"
      },
      "origin": {
        "node_id": "node-1",
        "boot_generation": "293a2951-5ba0-482c-91c7-0a0c72a5ce4b",
        "started_at": 1_700_000_000_000_u64
      },
      "presence_revision": 11,
      "occupancy_version": 7,
      "member_changes": [{
        "action": "enter",
        "connection_id": "connection-1",
        "client_id": "client-1",
        "data": {"status": "online"},
        "message_id": "message-1",
        "timestamp": 1_700_000_000_000_u64
      }],
      "occupancy": null,
      "occurred_at": 1_700_000_000_001_u64
    }))
    .expect("canonical presence change must deserialize");

    let committed = CommittedPresenceEvent::new(event_id, change);
    let message = EventMessage::try_from(&committed).expect("event message must serialize");

    assert_eq!(committed.event_id(), event_id);
    assert_eq!(message.event_id(), event_id);
    assert_eq!(message.event_name(), PresenceChannelChanged::NAME);
    assert_eq!(message.schema_version(), PresenceChannelChanged::VERSION);

    let decoded = message
      .decode_event::<PresenceChannelChanged>()
      .expect("event message must decode to the canonical presence change");

    assert_eq!(decoded.presence_revision, Some(11));
    assert_eq!(decoded.occupancy_version, 7);
    assert_eq!(decoded.member_changes.len(), 1);
    assert_eq!(
      decoded.member_changes[0].action,
      PresenceChangeAction::Enter
    );
  }
}
