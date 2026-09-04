use crate::{OccupancyMetrics, PresenceMember};

#[derive(Debug, Clone)]
pub struct PresenceSnapshot {
  /// Список участников Presence на момент создания снимка.
  pub members: Vec<PresenceMember>,

  /// Ревизия Presence, которой соответствует список участников.
  pub presence_revision: u64,

  /// Версия Occupancy, которой соответствуют метрики.
  pub occupancy_version: u64,

  /// Метрики Occupancy на момент создания снимка.
  pub occupancy: OccupancyMetrics,
}