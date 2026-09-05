use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use super::channel::ChannelState;
use super::IndividualDetachOutcome;
use crate::{AttachCommand, AttachmentTracking, ChannelAttachOutcome, ChannelKey, ChannelStateStoreError, CommittedChannelTransition, CommittedPresenceEvent, DetachCommand, PresenceChangeAction, PresenceChannelChanged, PresenceMemberChange, PresenceSnapshot};
use crate::channel::presence::PresenceOperationRecord;
use crate::connection::{ConnectionKey, DisconnectConnectionCommand};

/// Состояние локального хранилища каналов.
#[derive(Default)]
pub struct MemoryStoreState {
  /// Состояние каналов.
  channels: HashMap<ChannelKey, ChannelState>,

  /// Каналы, к которым присоединено каждое соединение.
  connection_channels: HashMap<ConnectionKey, HashSet<ChannelKey>>,

  /// Результаты обработанных Presence-команд, сгруппированные
  /// по соединению и `msg_serial`.
  presence_operations: HashMap<ConnectionKey, HashMap<u64, PresenceOperationRecord>>,
}

impl MemoryStoreState {

  /// Возвращает snapshot канала, не изменяя состояние хранилища.
  pub fn channel_snapshot(&self, channel: &ChannelKey) -> PresenceSnapshot {
    self
      .channels
      .get(channel)
      .map(ChannelState::snapshot)
      .unwrap_or_else(|| ChannelState::default().snapshot())
  }

  fn attach_exact(
    &mut self,
    command: AttachCommand,
  ) -> Result<ChannelAttachOutcome, ChannelStateStoreError> {
    if command.accounting != AttachmentTracking::Individual {
      return Err(ChannelStateStoreError::InvalidRequest {
        message: "individual attachment accounting is required".to_owned(),
      });
    }

    if !command.channel.belongs_to_application(&command.actor.application_id) {
      return Err(ChannelStateStoreError::InvalidRequest {
        message: "channel and connection belong to different applications".to_owned(),
      });
    }

    let connection_key = ConnectionKey::from(&command.actor);
    let channel = command.channel.clone();

    let attachment = command.to_attachment();

    let channel_state = self
      .channels
      .entry(channel.clone())
      .or_default();

    let occupancy_change = channel_state.save_attachment(attachment.clone())?;

    let snapshot = channel_state.snapshot();

    let event = occupancy_change.map(|occupancy| {
      CommittedPresenceEvent::new(
        Uuid::new_v4(),
        PresenceChannelChanged {
          channel: channel.clone(),
          origin: command.actor.node_instance,
          presence_revision: None,
          occupancy_version: snapshot.occupancy_version,
          member_changes: Vec::new(),
          occupancy: Some(occupancy),
          occurred_at: command.request_time,
        },
      )
    });

    let transition = CommittedChannelTransition {
      presence_revision: None,
      occupancy_version: snapshot.occupancy_version,
      event,
    };

    self
      .connection_channels
      .entry(connection_key)
      .or_default()
      .insert(channel);

    Ok(ChannelAttachOutcome {
      attachment,
      snapshot,
      transition,
      occupancy_shard_baseline: None,
    })
  }

  /// Завершает работу соединения с одним каналом.
  ///
  /// Вместе с attachment удаляет всех Presence-участников этого соединения.
  /// Повторный вызов для уже удалённого attachment считается успешным.
  pub fn detach(&mut self, command: DetachCommand) -> Result<CommittedChannelTransition, ChannelStateStoreError> {
    if !command.channel.belongs_to_application(&command.actor.application_id) {
      return Err(ChannelStateStoreError::InvalidRequest {
        message: "channel and connection belong to different applications".to_owned(),
      });
    }

    let connection_key = ConnectionKey::from(&command.actor);
    let channel = command.channel;

    let outcome = match self.channels.get_mut(&channel) {
      Some(channel_state) => channel_state.detach_individual(&command.actor)?,
      None => IndividualDetachOutcome::NotAttached {
        occupancy_version: 0,
      },
    };

    self.remove_connection_channel(&connection_key, &channel);

    match outcome {
      IndividualDetachOutcome::NotAttached { occupancy_version } => {
        Ok(CommittedChannelTransition {
          presence_revision: None,
          occupancy_version,
          event: None,
        })
      }
      IndividualDetachOutcome::Detached {
        removed_members,
        presence_revision,
        occupancy_version,
        occupancy_change,
      } => {
        let event_id = Uuid::new_v4();

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
            channel,
            origin: command.actor.node_instance,
            presence_revision,
            occupancy_version,
            member_changes,
            occupancy: Some(occupancy_change),
            occurred_at: command.request_time,
          },
        );

        Ok(CommittedChannelTransition {
          presence_revision,
          occupancy_version,
          event: Some(event),
        })
      }
    }
  }

  /// Удаляет соединение из всех его каналов и очищает журнал Presence.
  ///
  /// Все каналы проверяются до первого изменения состояния.
  /// Повторное отключение возвращает пустой список переходов.
  pub fn disconnect(
    &mut self,
    command: DisconnectConnectionCommand,
  ) -> Result<Vec<CommittedChannelTransition>, ChannelStateStoreError> {
    let connection_key = ConnectionKey::from(&command.actor);

    // Копируем ключи: detach будет изменять обратный индекс.
    let mut channels = self
      .connection_channels
      .get(&connection_key)
      .map(|channels| channels.iter().cloned().collect::<Vec<_>>())
      .unwrap_or_default();

    channels.sort_by(|left, right| left.channel.cmp(&right.channel));

    // Проверяем все каналы до удаления первого attachment.
    for channel in &channels {
      if !channel.belongs_to_application(&command.actor.application_id) {
        return Err(ChannelStateStoreError::InvalidRequest {
          message: "channel and connection belong to different applications"
            .to_owned(),
        });
      }

      if let Some(state) = self.channels.get(channel) {
        state.check_individual_detach(&command.actor)?;
      }
    }

    let mut transitions = Vec::with_capacity(channels.len());

    for channel in channels {
      let transition = self.detach(DetachCommand {
        channel,
        actor: command.actor.clone(),
        request_time: command.request_time,
      })?;

      transitions.push(transition);
    }

    self.connection_channels.remove(&connection_key);
    self.presence_operations.remove(&connection_key);

    Ok(transitions)
  }

  /// Удаляет связь соединения с каналом из обратного индекса.
  fn remove_connection_channel(&mut self, connection: &ConnectionKey, channel: &ChannelKey) {
    let remove_connection = self
      .connection_channels
      .get_mut(connection)
      .is_some_and(|channels| {
        channels.remove(channel);
        channels.is_empty()
      });

    if remove_connection {
      self.connection_channels.remove(connection);
    }
  }

}
