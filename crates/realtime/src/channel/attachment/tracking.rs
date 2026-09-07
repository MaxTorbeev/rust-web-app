use serde::{Deserialize, Serialize};

/// Способ учёта Attachment в состоянии канала.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AttachmentTracking {
  /// Attachment сохраняется как отдельная запись и самостоятельно
  /// участвует в расчёте Occupancy канала.
  Individual,

  /// Attachment учитывается в агрегированных счётчиках экземпляра ноды.
  /// Его Occupancy-метрики добавляются к общим счётчикам этого канала.
  ///
  /// Aggregated предназначен для неидентифицированных read-only соединений
  /// с большим количеством подключений, например зрителей публичного канала.
  /// * не создаёт PresenceMember;
  /// * не может выполнять ENTER, UPDATE и LEAVE;
  /// * изменяет только подходящие Occupancy-счётчики;
  Aggregated,
}
