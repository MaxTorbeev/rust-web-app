use serde::{Deserialize, Serialize};

/// Доменный отказ Presence-команды.
///
/// Отказ не меняет состояние канала. Варианты, описывающие результат самой
/// операции, записываются в журнал операций и воспроизводятся при повторе.
/// Варианты, описывающие состояние журнала (`ConflictingReplay`,
/// `StaleOperation`, `ConnectionClosed`), в журнал не попадают.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PresenceRejection {
  /// Соединение не присоединено к каналу.
  NotAttached,

  /// Attachment соединения не содержит режим `Presence`.
  PresenceModeNotEnabled,

  /// Соединение не идентифицировано: `client_id` отсутствует и в сообщении,
  /// и у соединения.
  UnidentifiedConnection,

  /// Политика соединения не разрешает указанный `client_id`.
  ClientIdNotAllowed { client_id: String },

  /// `UPDATE` или `LEAVE` для участника, которого нет в канале.
  InvalidMemberState,

  /// Повтор `msg_serial` с другим содержимым запроса.
  ConflictingReplay,

  /// `msg_serial` старше окна хранения журнала: результат операции уже
  /// вытеснен и не может быть воспроизведён; новой операцией повтор не станет.
  StaleOperation,

  /// Соединение авторитетно завершено; новые операции не принимаются.
  ConnectionClosed,
}
