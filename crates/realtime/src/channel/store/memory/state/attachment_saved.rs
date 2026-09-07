use super::individual_detach_outcome::server_leave_changes;
use crate::{
  AttachCommand, CommittedChannelTransition, CommittedPresenceEvent, OccupancyChange,
  PresenceChannelChanged, PresenceMember,
};

/// Результат сохранения attachment-а в состоянии канала.
///
/// Повторный attach может сузить режимы: если новый attachment лишился режима
/// `Presence`, участники этого соединения удаляются тем же переходом — иначе в
/// канале остался бы участник, которому store не разрешил бы ни `ENTER`, ни
/// `LEAVE`. Удаление публикуется server-generated `Leave` и одной ревизией.
pub(super) struct AttachmentSaved {
  /// Изменение Occupancy, если сохранение повлияло на метрики.
  pub(super) occupancy_change: Option<OccupancyChange>,

  /// Участники, удалённые из-за потери режима `Presence`, в порядке `client_id`.
  pub(super) removed_members: Vec<PresenceMember>,

  /// Новая ревизия Presence, если участники были удалены.
  pub(super) presence_revision: Option<u64>,
}

impl AttachmentSaved {
  /// Формирует transition из результата сохранения и контекста команды.
  ///
  /// `occupancy_version` — текущая версия канала после сохранения: она нужна и
  /// для `Unchanged`, и для события.
  pub(super) fn into_transition(
    self,
    command: &AttachCommand,
    occupancy_version: u64,
  ) -> CommittedChannelTransition {
    // Удаление участников всегда меняет `presence_members`, поэтому событие без
    // изменения occupancy может быть только пустым.
    let Some(occupancy) = self.occupancy_change else {
      debug_assert!(self.removed_members.is_empty(), "removed members without an occupancy change");

      return CommittedChannelTransition::Unchanged { occupancy_version };
    };

    let event = CommittedPresenceEvent::new(
      command.event_id,
      PresenceChannelChanged {
        channel: command.channel.clone(),
        origin: command.actor.node_instance.clone(),
        presence_revision: self.presence_revision,
        occupancy_version,
        member_changes: server_leave_changes(command.event_id, self.removed_members, command.request_time),
        occupancy: Some(occupancy),
        occurred_at: command.request_time,
      },
    );

    CommittedChannelTransition::Changed(event)
  }
}
