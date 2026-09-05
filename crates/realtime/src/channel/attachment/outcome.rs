use crate::channel::attachment::Attachment;
use crate::{CommittedChannelTransition, OccupancyShardBaseline, PresenceSnapshot};

/// Результат подготовки `ATTACH` в хранилище.
///
/// Для индивидуально учитываемого подключения хранилище сохраняет
/// [`Attachment`] и возвращает состояние канала после этой операции.
///
/// Для агрегированно учитываемого подключения отдельная запись не создаётся:
/// хранилище возвращает снимок канала и счётчики текущего экземпляра ноды,
/// уже включённые в этот снимок.
pub struct ChannelAttachOutcome {
  /// Параметры работы соединения с каналом, сохранённые хранилищем.
  pub attachment: Attachment,

  /// Снимок Presence и Occupancy после сохранения attachment.
  pub snapshot: PresenceSnapshot,

  /// Результат изменения состояния, зафиксированного этой операцией.
  ///
  /// При идемпотентном повторе может не содержать нового события.
  pub transition: CommittedChannelTransition,

  /// Счётчики `NodeInstance`, обслуживающего соединение, которые уже входят
  /// в `snapshot.occupancy`.
  ///
  /// `None`, если подключение учитывается индивидуально.
  pub occupancy_shard_baseline: Option<OccupancyShardBaseline>,
}
