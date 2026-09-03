use support::NodeInstance;
use crate::{ChannelMode, ConnectionId, OccupancySubscription};

/// Состояние подключения
#[derive(Debug, Clone)]
pub struct Attachment {
  pub connection_id: ConnectionId,
  pub owner: NodeInstance,
  pub effective_modes: Vec<ChannelMode>,
  pub occupancy: Option<OccupancySubscription>,
}