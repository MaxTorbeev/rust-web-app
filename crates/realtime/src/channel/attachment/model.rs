use crate::{AttachmentTracking, ChannelMode, ConnectionId, OccupancySubscription};
use serde::{Deserialize, Serialize};
use support::NodeInstance;

/// Запись о том, что Realtime-соединение присоединено к каналу,
/// и параметры этого присоединения.
///
/// Сериализуемый формат — формат хранения attachment в Redis; эталон
/// зафиксирован snapshot-тестом.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
  pub connection_id: ConnectionId,
  /// Экземпляр ноды, обслуживающий соединение.
  pub node_instance: NodeInstance,
  pub accounting: AttachmentTracking,
  pub effective_modes: Vec<ChannelMode>,
  pub occupancy: Option<OccupancySubscription>,
}

impl Attachment {
  /// Проверяет, включён ли ChannelMode для этого присоединения.
  pub fn has_mode(&self, mode: ChannelMode) -> bool {
    self.effective_modes.contains(&mode)
  }

  pub const fn is_individual(&self) -> bool {
    matches!(self.accounting, AttachmentTracking::Individual)
  }

  pub const fn is_aggregated(&self) -> bool {
    matches!(self.accounting, AttachmentTracking::Aggregated)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use support::{BootGeneration, NodeId, timestamp::Timestamp};
  use uuid::Uuid;

  /// Формат хранения attachment: camelCase, режимы — списком строк,
  /// подписка Occupancy — canonical wire-значением.
  #[test]
  fn attachment_wire_format() {
    let attachment = Attachment {
      connection_id: serde_json::from_value(serde_json::json!("connection-1")).unwrap(),
      node_instance: NodeInstance::new(
        NodeId::try_new("node-1").unwrap(),
        BootGeneration::from_uuid(Uuid::parse_str("293a2951-5ba0-482c-91c7-0a0c72a5ce4b").unwrap()),
        Timestamp::from_millis(1_700_000_000_000),
      ),
      accounting: AttachmentTracking::Individual,
      effective_modes: vec![ChannelMode::Subscribe, ChannelMode::PresenceSubscribe],
      occupancy: Some(OccupancySubscription::Metrics),
    };

    insta::assert_json_snapshot!("attachment", attachment);

    let decoded: Attachment =
      serde_json::from_value(serde_json::to_value(&attachment).unwrap()).unwrap();
    assert_eq!(decoded.effective_modes, attachment.effective_modes);
    assert_eq!(decoded.occupancy, attachment.occupancy);
    assert!(decoded.is_individual());
  }
}
