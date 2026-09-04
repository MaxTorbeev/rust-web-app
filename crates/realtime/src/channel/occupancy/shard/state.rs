use crate::ChannelKey;
use support::NodeInstance;

/// Текущие счётчики агрегированного Occupancy одного канала
/// на конкретном экземпляре ноды.
#[derive(Debug)]
pub struct OccupancyShardState {
  node_instance: NodeInstance,
  channel: ChannelKey,

  /// Версия изменяется при каждом изменении счётчиков.
  version: u64,

  connections: u64,
  subscribers: u64,
  presence_subscribers: u64,

  /// Показывает, что текущее состояние ещё не отправлено в хранилище.
  dirty: bool,
}

impl OccupancyShardState {
  pub fn new(node_instance: NodeInstance, channel: ChannelKey) -> Self {
    Self {
      node_instance,
      channel,
      version: 0,
      connections: 0,
      subscribers: 0,
      presence_subscribers: 0,
      dirty: false,
    }
  }
}
