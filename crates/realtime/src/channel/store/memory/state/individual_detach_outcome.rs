use crate::{
  CommittedChannelTransition, CommittedPresenceEvent, DetachCommand, OccupancyChange,
  PresenceChangeAction, PresenceChannelChanged, PresenceMember, PresenceMemberChange,
};

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
        let event_id = command.event_id;

        let member_changes = removed_members
          .into_iter()
          .enumerate()
          .map(|(index, member)| PresenceMemberChange {
            action: PresenceChangeAction::Leave,
            connection_id: member.connection_id,
            client_id: member.client_id,
            data: member.data,
            message_id: format!("server:{event_id}:{index}"),
            timestamp: command.request_time,
          })
          .collect();

        let event = CommittedPresenceEvent::new(
          event_id,
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
