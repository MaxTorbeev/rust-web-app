use crate::ChannelKey;
use support::NodeInstance;

/// Полный снимок вклада одного экземпляра ноды
/// в агрегированный Occupancy канала.
///
/// Содержит абсолютные значения счётчиков, поэтому новая версия заменяет
/// предыдущий вклад этого экземпляра ноды, а не прибавляется к нему.
#[derive(Debug, Clone)]
pub struct OccupancyShardSnapshot {
  pub node_instance: NodeInstance,
  pub channel: ChannelKey,
  pub version: u64,
  pub connections: u64,
  pub subscribers: u64,
  pub presence_subscribers: u64,
}
