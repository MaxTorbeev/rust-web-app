use crate::{ChannelKey, PresenceActor, PresenceMutationAction};
use serde_json::Value;
use support::timestamp::Timestamp;

#[derive(Debug, Clone)]
pub struct PresenceBatchItem {
  pub action: PresenceMutationAction,

  /// Идентификатор участника после применения протокольного правила подстановки.
  ///
  /// `None` означает, что идентификатор отсутствует и в сообщении, и у
  /// соединения. Хранилище должно вернуть доменный отказ, не изменяя Presence.
  pub client_id: Option<String>,

  pub data: Option<Value>,
}

/// Атомарная команда изменения Presence одного канала.
#[derive(Debug, Clone)]
pub struct PresenceBatchCommand {
  pub channel: ChannelKey,
  pub actor: PresenceActor,
  pub items: Vec<PresenceBatchItem>,
  /// Последовательный номер клиентской операции.
  pub msg_serial: u64,
  /// Хеш данных запроса.
  ///
  /// Позволяет отличить повтор той же операции от другого запроса,
  /// отправленного с тем же `msg_serial`.
  pub request_fingerprint: String,

  /// Время получения запроса сервером.
  pub request_time: Timestamp,
}

impl PresenceBatchCommand {
  /// Возвращает стабильный идентификатор сообщения для элемента пакета.
  ///
  /// Повтор команды с тем же `connection_id` и `msg_serial` создаёт те же
  /// идентификаторы сообщений.
  pub fn message_id(&self, index: usize) -> String {
    debug_assert!(index < self.items.len());

    format!(
      "{}:{}:{index}",
      self.actor.connection_actor.connection_id.as_str(),
      self.msg_serial,
    )
  }
}
