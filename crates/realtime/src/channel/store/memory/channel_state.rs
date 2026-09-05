use std::collections::HashMap;
use uuid::Uuid;
use support::NodeInstance;
use crate::{Attachment, AttachmentTracking, ChannelKey, ChannelMode, ChannelStateStoreError, CommittedChannelTransition, CommittedPresenceEvent, ConnectionId, DetachCommand, OccupancyChange, OccupancyMetrics, PresenceChangeAction, PresenceChannelChanged, PresenceMember, PresenceMemberChange, PresenceSnapshot};
use crate::channel::store::memory::store_state::ConnectionKey;
use crate::connection::ConnectionActor;

/// Актуальные счётчики Occupancy, сохранённые для экземпляра ноды.
#[derive(Clone, Debug)]
struct StoredOccupancyShard {
  /// Последняя принятая версия счётчиков.
  version: u64,
  connections: u64,
  subscribers: u64,
  presence_subscribers: u64,
}

/// Внутреннее состояние одного канала в локальном хранилище.
#[derive(Default)]
pub(super) struct ChannelState {
  /// Активные присоединения к этому каналу по идентификатору соединения.
  attachments: HashMap<ConnectionId, Attachment>,

  /// Участники Presence, сгруппированные по соединению и `client_id`.
  members: HashMap<ConnectionId, HashMap<String, PresenceMember>>,

  /// Последние абсолютные счётчики Occupancy каждого экземпляра ноды.
  occupancy_shards: HashMap<NodeInstance, StoredOccupancyShard>,

  /// Текущая ревизия списка участников Presence.
  presence_revision: u64,

  /// Текущая версия метрик Occupancy.
  occupancy_version: u64,
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

impl ChannelState {
  /// Рассчитывает текущие метрики Occupancy канала.
  pub(super) fn occupancy(&self) -> OccupancyMetrics {
    let mut metrics = OccupancyMetrics {
      connections: 0,
      publishers: 0,
      subscribers: 0,
      presence_connections: 0,
      presence_subscribers: 0,
      presence_members: self
        .members
        .values()
        .map(|members| members.len() as u64)
        .sum(),
    };

    for attachment in self.attachments.values() {
      // Предотвратить двойной подсчёт агрегированного attachment.
      if attachment.accounting == AttachmentTracking::Aggregated {
        continue;
      }

      metrics.connections += 1;

      if attachment.has_mode(ChannelMode::Publish) {
        metrics.publishers += 1;
      }

      if attachment.has_mode(ChannelMode::Subscribe) {
        metrics.subscribers += 1;
      }

      if attachment.has_mode(ChannelMode::Presence) {
        metrics.presence_connections += 1;
      }

      if attachment.has_mode(ChannelMode::PresenceSubscribe) {
        metrics.presence_subscribers += 1;
      }
    }

    for shard in self.occupancy_shards.values() {
      metrics.connections += shard.connections;
      metrics.subscribers += shard.subscribers;
      metrics.presence_subscribers += shard.presence_subscribers;
    }

    metrics
  }

  /// Создаёт snapshot текущего состояния Presence и Occupancy.
  pub(super) fn snapshot(&self) -> PresenceSnapshot {
    let members = self
      .members
      .values()
      .flat_map(|members| members.values())
      .cloned()
      .collect();

    PresenceSnapshot {
      members,
      presence_revision: self.presence_revision,
      occupancy_version: self.occupancy_version,
      occupancy: self.occupancy(),
    }
  }

  pub(super) const fn occupancy_version(&self) -> u64 {
    self.occupancy_version
  }

  /// Сохраняет или обновляет attachment соединения.
  ///
  /// Возвращает изменение Occupancy, если сохранение повлияло на метрики канала.
  pub(super) fn save_attachment(
    &mut self,
    attachment: Attachment,
  ) -> Result<Option<OccupancyChange>, ChannelStateStoreError> {
    self.validate_attachment(&attachment)?;

    let connection_id = attachment.connection_id.clone();
    let before = self.occupancy();

    let previous = self
      .attachments
      .insert(connection_id.clone(), attachment);

    let after = self.occupancy();
    let change = OccupancyChange::between(before, after);

    if change.is_some() {
      if let Err(error) = self.increment_occupancy_version() {
        self.restore_attachment(connection_id, previous);
        return Err(error);
      }
    }

    Ok(change)
  }

  pub(super) fn detach_individual(
    &mut self,
    actor: &ConnectionActor,
  ) -> Result<IndividualDetachOutcome, ChannelStateStoreError> {
    let Some(attachment) = self.attachments.get(&actor.connection_id) else {
      return Ok(IndividualDetachOutcome::NotAttached {
        occupancy_version: self.occupancy_version,
      });
    };

    Self::validate_individual_detach(attachment, actor)?;

    let has_members = self
      .members
      .get(&actor.connection_id)
      .is_some_and(|members| !members.is_empty());

    // Обе версии проверяем до изменения состояния.
    let next_presence_revision = if has_members {
      Some(self.next_presence_revision()?)
    } else {
      None
    };
    let next_occupancy_version = self.next_occupancy_version()?;

    let before = self.occupancy();

    self.attachments.remove(&actor.connection_id);
    let removed_members = self.remove_members(&actor.connection_id);

    if let Some(revision) = next_presence_revision {
      self.presence_revision = revision;
    }
    self.occupancy_version = next_occupancy_version;

    let after = self.occupancy();
    let occupancy_change = OccupancyChange::between(before, after)
      .expect("removing an individual attachment must change occupancy");

    Ok(IndividualDetachOutcome::Detached {
      removed_members,
      presence_revision: next_presence_revision,
      occupancy_version: next_occupancy_version,
      occupancy_change,
    })
  }

  fn validate_individual_detach(attachment: &Attachment, actor: &ConnectionActor) -> Result<(), ChannelStateStoreError> {
    if attachment.node_instance != actor.node_instance {
      return Err(ChannelStateStoreError::Conflict {
        message: format!(
          "attachment {} belongs to another node instance",
          actor.connection_id.as_str(),
        ),
      });
    }

    if !attachment.is_individual() {
      return Err(ChannelStateStoreError::Internal {
        message: "aggregated attachment cannot be removed as individual".to_owned(),
      });
    }

    Ok(())
  }

  fn next_presence_revision(&self) -> Result<u64, ChannelStateStoreError> {
    self
      .presence_revision
      .checked_add(1)
      .ok_or_else(|| ChannelStateStoreError::Internal {
        message: "presence revision overflow".to_owned(),
      })
  }

  fn next_occupancy_version(&self) -> Result<u64, ChannelStateStoreError> {
    self
      .occupancy_version
      .checked_add(1)
      .ok_or_else(|| ChannelStateStoreError::Internal {
        message: "occupancy version overflow".to_owned(),
      })
  }

  fn remove_members(&mut self, connection_id: &ConnectionId) -> Vec<PresenceMember> {
    let mut removed_members = self
      .members
      .remove(connection_id)
      .unwrap_or_default()
      .into_values()
      .collect::<Vec<_>>();

    // HashMap не гарантирует порядок. Событие должно быть детерминированным.
    removed_members.sort_by(|left, right| left.client_id.cmp(&right.client_id));

    removed_members
  }

  fn validate_attachment(&self, attachment: &Attachment) -> Result<(), ChannelStateStoreError> {
    if let Some(current) = self.attachments.get(&attachment.connection_id) {
      if current.node_instance != attachment.node_instance {
        return Err(ChannelStateStoreError::Conflict {
          message: format!("attachment {} belongs to another node instance", attachment.connection_id.as_str()),
        });
      }
    }

    Ok(())
  }

  fn increment_occupancy_version(&mut self) -> Result<(), ChannelStateStoreError> {
    let next_version = self
      .occupancy_version
      .checked_add(1)
      .ok_or_else(|| ChannelStateStoreError::Internal {
        message: "occupancy version overflow".to_owned(),
      })?;

    self.occupancy_version = next_version;

    Ok(())
  }

  fn restore_attachment(&mut self, connection_id: ConnectionId, previous: Option<Attachment>) {
    match previous {
      Some(previous) => {
        self.attachments.insert(connection_id, previous);
      }
      None => {
        self.attachments.remove(&connection_id);
      }
    }
  }
}
