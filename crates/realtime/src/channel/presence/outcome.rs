use crate::channel::attachment::Attachment;
use crate::{CommittedTransition, OccupancyShardBaseline, PresenceRejection, PresenceSnapshot};
use serde::{Deserialize, Serialize};

/// Результат подготовки ATTACH в хранилище.
///
/// Для индивидуально учитываемого подключения хранилище сохраняет
/// [`Attachment`] и возвращает состояние канала после этой операции.
///
/// Для агрегированно учитываемого подключения отдельная запись не создаётся:
/// хранилище возвращает снимок канала и счётчики текущего экземпляра ноды,
/// уже включённые в этот снимок.
pub struct PresenceAttachOutcome {
  /// Параметры работы соединения с каналом, сохранённые хранилищем.
  pub attachment: Attachment,

  /// Снимок Presence и Occupancy после сохранения attachment.
  pub snapshot: PresenceSnapshot,

  /// Результат перехода состояния, зафиксированного этой операцией.
  ///
  /// При идемпотентном повторе может не содержать нового события.
  pub transition: CommittedTransition,

  /// Счётчики `NodeInstance`, обслуживающего соединение, которые уже входят
  /// в `snapshot.occupancy`.
  ///
  /// `None`, если подключение учитывается индивидуально.
  pub occupancy_shard_baseline: Option<OccupancyShardBaseline>,
}

/// Итог обработки команды изменения Presence.
///
/// Команда либо полностью фиксируется и возвращает описание произошедшего
/// перехода, либо отклоняется без изменения состояния. Частичное применение
/// элементов одной команды не допускается.
///
/// Инфраструктурные ошибки хранилища в этот тип не входят и возвращаются через
/// `Result<_, PresenceStoreError>`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PresenceMutationOutcome {
  /// Все изменения команды зафиксированы.
  Committed(CommittedTransition),

  /// Команда отклонена без изменения Presence.
  Rejected(PresenceRejection),
}

/// Результат обработки команды изменения Presence.
///
/// Содержит доменный результат операции и указывает, была ли команда выполнена
/// впервые или хранилище вернуло результат ранее обработанной команды с тем же
/// ключом дедупликации.
///
/// Повторная обработка не изменяет Presence, не создаёт новую ревизию и событие.
/// Клиенту при этом должен быть возвращён тот же ACK или NACK, что и при первой
/// обработке команды.
#[derive(Clone, Debug)]
pub struct PresenceMutationReceipt {
  /// Зафиксированный или отклонённый результат операции.
  pub outcome: PresenceMutationOutcome,

  /// `true`, если результат загружен из журнала ранее обработанных операций;
  /// `false`, если команда была обработана впервые.
  pub replayed: bool,
}
