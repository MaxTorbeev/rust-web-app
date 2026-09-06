use crate::connection::ConnectionActor;
use crate::{Attachment, AttachmentTracking, ChannelKey, ChannelMode, OccupancySubscription};
use support::timestamp::Timestamp;
use uuid::Uuid;

/// Команда на начало работы соединения с каналом.
#[derive(Debug, Clone)]
pub struct AttachCommand {
  pub channel: ChannelKey,
  pub actor: ConnectionActor,
  /// Способ хранения и учёта attachment.
  pub accounting: AttachmentTracking,
  /// Effective (server calculated) modes and requested occupancy subscription.
  pub effective_modes: Vec<ChannelMode>,
  pub occupancy: Option<OccupancySubscription>,
  /// Server timestamp in ms.
  pub request_time: Timestamp,
  /// Кандидат `event_id` события, которое создаст эта команда.
  ///
  /// Свежий `support::fresh_uuid` на каждый вызов. Хранилище обязано
  /// использовать его для нового события и не имеет права генерировать свой;
  /// при воспроизведении уже обработанной операции возвращается исходное
  /// событие, а кандидат игнорируется.
  pub event_id: Uuid,
}

impl AttachCommand {
  /// Создаёт запись attachment из параметров команды.
  pub fn to_attachment(&self) -> Attachment {
    Attachment {
      connection_id: self.actor.connection_id.clone(),
      node_instance: self.actor.node_instance.clone(),
      accounting: self.accounting,
      effective_modes: self.effective_modes.clone(),
      occupancy: self.occupancy.clone(),
    }
  }
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
  /// Кандидат `event_id` события, которое создаст эта команда.
  ///
  /// Свежий `support::fresh_uuid` на каждый вызов. Хранилище обязано
  /// использовать его для нового события и не имеет права генерировать свой;
  /// при воспроизведении уже обработанной операции возвращается исходное
  /// событие, а кандидат игнорируется.
  pub event_id: Uuid,
}
