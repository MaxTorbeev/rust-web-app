use crate::{
  CommittedChannelTransition, CommittedPresenceEvent, DetachCommand, OccupancyChange,
  PresenceChangeAction, PresenceChannelChanged, PresenceMember, PresenceMemberChange,
};
use support::timestamp::Timestamp;
use uuid::Uuid;

/// Server-generated `Leave` для участников, удалённых не клиентской командой
/// (detach, disconnect, потеря режима `Presence` при повторном attach).
///
/// `message_id` имеет вид `server:<eventId>:<index>` и не начинается с
/// `connection_id` участника, чтобы SDK не принял его за клиентскую операцию.
pub(super) fn server_leave_changes(
  event_id: Uuid,
  removed_members: Vec<PresenceMember>,
  timestamp: Timestamp,
) -> Vec<PresenceMemberChange> {
  removed_members
    .into_iter()
    .enumerate()
    .map(|(index, member)| PresenceMemberChange {
      action: PresenceChangeAction::Leave,
      connection_id: member.connection_id,
      client_id: member.client_id,
      data: member.data,
      message_id: format!("server:{event_id}:{index}"),
      timestamp,
    })
    .collect()
}

/// Результат удаления индивидуального attachment.
pub(super) enum IndividualDetachOutcome {
  NotAttached {
    occupancy_version: u64,
  },
  Detached {
    removed_members: Vec<PresenceMember>,
    presence_revision: Option<u64>,
    occupancy_version: u64,
    occupancy_change: OccupancyChange,
  },
}

impl IndividualDetachOutcome {
  /// Формирует transition из результата удаления и контекста команды.
  pub fn into_transition(self, command: DetachCommand) -> CommittedChannelTransition {
    match self {
      Self::NotAttached { occupancy_version } => {
        CommittedChannelTransition::Unchanged { occupancy_version }
      }
      Self::Detached {
        removed_members,
        presence_revision,
        occupancy_version,
        occupancy_change,
      } => {
        let member_changes =
          server_leave_changes(command.event_id, removed_members, command.request_time);

        let event = CommittedPresenceEvent::new(
          command.event_id,
          PresenceChannelChanged {
            channel: command.channel,
            origin: command.actor.node_instance,
            presence_revision,
            occupancy_version,
            member_changes,
            occupancy: Some(occupancy_change),
            occurred_at: command.request_time,
          },
        );

        CommittedChannelTransition::Changed(event)
      }
    }
  }
}
