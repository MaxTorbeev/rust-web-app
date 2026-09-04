use crate::{ChannelKey, PresenceActor, PresenceMutationAction};
use serde_json::Value;
use support::timestamp::Timestamp;

#[derive(Debug, Clone)]
pub struct PresenceBatchItem {
  pub action: PresenceMutationAction,
  pub client_id: String,
  pub message_id: String,
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
  /// Хеш нормализованного содержимого запроса.
  pub normalized_request_hash: String,
  /// Время получения запроса сервером.
  pub request_time: Timestamp,
}
