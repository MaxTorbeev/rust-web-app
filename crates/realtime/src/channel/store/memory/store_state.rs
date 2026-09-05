use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use super::channel_state::ChannelState;
use crate::{ApplicationId, AttachCommand, AttachmentAccounting, ChannelAttachOutcome, ChannelKey, ChannelStateStoreError, CommittedChannelTransition, CommittedPresenceEvent, ConnectionId, PresenceChannelChanged, PresenceMutationOutcome, PresenceSnapshot};
use crate::connection::ConnectionActor;

/// Сохранённый результат обработанной Presence-команды.
#[derive(Clone, Debug)]
struct PresenceOperationRecord {
  /// Хеш содержимого первоначальной команды.
  request_fingerprint: String,

  /// Результат, который необходимо вернуть при повторе команды.
  outcome: PresenceMutationOutcome,
}

/// Соединение в пределах приложения.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct ConnectionKey {
  application_id: ApplicationId,
  connection_id: ConnectionId,
}

/// Состояние локального хранилища каналов.
#[derive(Default)]
pub(super) struct MemoryStoreState {
  /// Состояние каналов.
  channels: HashMap<ChannelKey, ChannelState>,

  /// Каналы, к которым присоединено каждое соединение.
  connection_channels: HashMap<ConnectionKey, HashSet<ChannelKey>>,

  /// Результаты обработанных Presence-команд, сгруппированные
  /// по соединению и `msg_serial`.
  presence_operations: HashMap<ConnectionKey, HashMap<u64, PresenceOperationRecord>>,
}

impl From<&ConnectionActor> for ConnectionKey {
  fn from(actor: &ConnectionActor) -> Self {
    Self {
      application_id: actor.application_id.clone(),
      connection_id: actor.connection_id.clone(),
    }
  }
}

impl MemoryStoreState {

  /// Возвращает snapshot канала, не изменяя состояние хранилища.
  pub(super) fn channel_snapshot(&self, channel: &ChannelKey) -> PresenceSnapshot {
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
    if command.accounting != AttachmentAccounting::Individual {
      return Err(ChannelStateStoreError::InvalidRequest {
        message: "individual attachment accounting is required".to_owned(),
      });
    }

    if command.channel.application_id != command.actor.application_id {
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
}
