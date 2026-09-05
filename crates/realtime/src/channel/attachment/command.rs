use crate::connection::ConnectionActor;
use crate::{ChannelKey, ChannelMode, OccupancySubscription};
use support::timestamp::Timestamp;

/// Команда на начало работы соединения с каналом.
#[derive(Debug, Clone)]
pub struct AttachCommand {
  pub channel: ChannelKey,
  pub actor: ConnectionActor,
  /// Effective (server calculated) modes and requested occupancy subscription.
  pub effective_modes: Vec<ChannelMode>,
  pub occupancy: Option<OccupancySubscription>,
  /// Server timestamp in ms.
  pub request_time: Timestamp,
}

/// Команда завершения работы соединения с одним каналом.
#[derive(Debug, Clone)]
pub struct DetachCommand {
  /// Канал, работу с которым необходимо завершить соединение.
  pub channel: ChannelKey,
  /// Соединение и экземпляр ноды, обслуживающий его.
  pub actor: ConnectionActor,
  /// Время начала обработки команды сервером.
  pub request_time: Timestamp,
}
