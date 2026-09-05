use crate::{ChannelMode, ConnectionId, OccupancySubscription};
use support::NodeInstance;

/// Запись о том, что Realtime-соединение присоединено к каналу,
/// и параметры этого присоединения.
#[derive(Debug, Clone)]
pub struct Attachment {
  pub connection_id: ConnectionId,
  pub owner: NodeInstance,
  pub effective_modes: Vec<ChannelMode>,
  pub occupancy: Option<OccupancySubscription>,
}

impl Attachment {
  /// Проверяет, включён ли ChannelMode для этого присоединения.
  pub fn has_mode(&self, mode: ChannelMode) -> bool {
    self.effective_modes.contains(&mode)
  }
}