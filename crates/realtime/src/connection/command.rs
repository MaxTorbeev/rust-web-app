use crate::ChannelKey;
use crate::connection::ConnectionActor;
use support::derive_uuid;
use support::timestamp::Timestamp;
use uuid::Uuid;

/// Команда очистки состояния Presence после завершения соединения.
#[derive(Debug, Clone)]
pub struct DisconnectConnectionCommand {
  /// Контекст соединения: приложение, идентификатор и экземпляр ноды.
  pub actor: ConnectionActor,

  /// Время начала обработки отключения сервером.
  pub request_time: Timestamp,

  /// Кандидат `event_id` для событий этой команды: свежий `support::fresh_uuid`
  /// на каждый вызов.
  ///
  /// Disconnect создаёт по событию на канал, а число каналов заранее
  /// неизвестно, поэтому команда несёт одну базу, а идентификатор каждого
  /// канала выводится из неё детерминированно — см. [`Self::channel_event_id`].
  pub event_id: Uuid,
}

impl DisconnectConnectionCommand {
  /// `event_id` события detach для одного из каналов соединения.
  ///
  /// Разные каналы одной команды получают разные идентификаторы; разные
  /// команды — разные базы и, следовательно, разные идентификаторы для одного
  /// и того же канала. Хранилище обязано использовать именно это значение.
  pub fn channel_event_id(&self, channel: &ChannelKey) -> Uuid {
    derive_uuid(self.event_id, channel.channel.as_bytes())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ApplicationId;
  use support::{BootGeneration, NodeId, NodeInstance, fresh_uuid};

  fn command(event_id: Uuid) -> DisconnectConnectionCommand {
    DisconnectConnectionCommand {
      actor: ConnectionActor {
        application_id: ApplicationId::new("application-1"),
        connection_id: crate::ConnectionId::generate(),
        node_instance: NodeInstance::new(
          NodeId::try_new("test-node").expect("test node id must be valid"),
          BootGeneration::generate(),
          Timestamp::from_millis(1_700_000_000_000),
        ),
      },
      request_time: Timestamp::from_millis(1_700_000_000_000),
      event_id,
    }
  }

  fn channel(name: &str) -> ChannelKey {
    ChannelKey::new(ApplicationId::new("application-1"), name)
  }

  #[test]
  fn channel_event_ids_differ_per_channel_and_per_command() {
    let first = command(fresh_uuid());
    let second = command(fresh_uuid());

    assert_eq!(
      first.channel_event_id(&channel("room-1")),
      first.channel_event_id(&channel("room-1")),
    );
    assert_ne!(
      first.channel_event_id(&channel("room-1")),
      first.channel_event_id(&channel("room-2")),
    );
    assert_ne!(
      first.channel_event_id(&channel("room-1")),
      second.channel_event_id(&channel("room-1")),
    );
  }
}
