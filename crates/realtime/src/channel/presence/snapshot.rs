use crate::{OccupancyMetrics, PresenceMember};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresenceSnapshot {
  /// Список участников Presence на момент создания снимка.
  ///
  /// Порядок — часть контракта хранилища: участники отсортированы по
  /// `(connection_id, client_id)`. Это делает snapshot сравнимым между
  /// вызовами и реализациями и не зависит от порядка входа участников.
  pub members: Vec<PresenceMember>,

  /// Ревизия Presence, которой соответствует список участников.
  pub presence_revision: u64,

  /// Версия Occupancy, которой соответствуют метрики.
  pub occupancy_version: u64,

  /// Метрики Occupancy на момент создания снимка.
  pub occupancy: OccupancyMetrics,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn empty_snapshot_wire_format() {
    let snapshot = PresenceSnapshot {
      members: Vec::new(),
      presence_revision: 0,
      occupancy_version: 0,
      occupancy: OccupancyMetrics {
        connections: 0,
        publishers: 0,
        subscribers: 0,
        presence_connections: 0,
        presence_subscribers: 0,
        presence_members: 0,
      },
    };

    insta::assert_json_snapshot!("empty_snapshot", snapshot);
  }
}
