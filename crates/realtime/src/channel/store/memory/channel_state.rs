use std::collections::HashMap;
use support::NodeInstance;
use crate::{Attachment, ChannelMode, ConnectionId, OccupancyMetrics, PresenceMember};

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

impl ChannelState {
  /// Рассчитывает текущие метрики Occupancy канала.
  pub(super) fn occupancy(&self) -> OccupancyMetrics {
    let mut metrics = OccupancyMetrics {
      connections: self.attachments.len() as u64,
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
}
