use crate::{CommittedChannelTransition, PresenceSnapshot};

/// Результат `ATTACH` в хранилище.
///
/// Хранилище сохраняет attachment и возвращает состояние канала после этой
/// операции. Сам attachment не возвращается: вызывающий построил его из
/// команды и ничего нового о нём от хранилища не узнаёт.
pub struct ChannelAttachOutcome {
  /// Снимок Presence и Occupancy после сохранения attachment.
  pub snapshot: PresenceSnapshot,

  /// Результат изменения состояния, зафиксированного этой операцией.
  ///
  /// При идемпотентном повторе может не содержать нового события.
  pub transition: CommittedChannelTransition,
}
