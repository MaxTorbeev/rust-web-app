use crate::{AttachmentTracking, ChannelMode, ConnectionId, OccupancySubscription};
use support::NodeInstance;

/// Запись о том, что Realtime-соединение присоединено к каналу,
/// и параметры этого присоединения.
#[derive(Debug, Clone)]
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
