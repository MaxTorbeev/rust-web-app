use std::collections::{HashMap, HashSet};
use tokio::sync::Mutex;
use support::NodeInstance;
use crate::{ApplicationId, Attachment, ChannelKey, ConnectionId, PresenceMember, PresenceMutationOutcome};

/// Соединение в пределах приложения.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ConnectionKey {
  application_id: ApplicationId,
  connection_id: ConnectionId,
}

/// Актуальные счётчики Occupancy, сохранённые для экземпляра ноды.
#[derive(Clone, Debug)]
struct StoredOccupancyShard {
  /// Последняя принятая версия счётчиков.
  version: u64,
  connections: u64,
  subscribers: u64,
  presence_subscribers: u64,
}

/// Сохранённый результат обработанной Presence-команды.
#[derive(Clone, Debug)]
struct PresenceOperationRecord {
  /// Хеш содержимого первоначальной команды.
  request_fingerprint: String,

  /// Результат, который необходимо вернуть при повторе команды.
  outcome: PresenceMutationOutcome,
}

#[derive(Default)]
struct ChannelState {
  /// Активные записи присоединения к каналам по идентификатору соединения.
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

/// Состояние локального хранилища каналов.
#[derive(Default)]
struct MemoryStoreState {
  /// Состояние каналов.
  channels: HashMap<ChannelKey, ChannelState>,

  /// Каналы, к которым присоединено каждое соединение.
  connection_channels: HashMap<ConnectionKey, HashSet<ChannelKey>>,

  /// Результаты обработанных Presence-команд, сгруппированные
  /// по соединению и `msg_serial`.
  presence_operations: HashMap<ConnectionKey, HashMap<u64, PresenceOperationRecord>>,
}

/// Локальное хранилище состояния каналов.
pub struct MemoryChannelStore {
  state: Mutex<MemoryStoreState>,
}

impl MemoryChannelStore {
  pub fn new() -> Self {
    Self {
      state: Mutex::new(MemoryStoreState::default()),
    }
  }
}

impl Default for MemoryChannelStore {
  fn default() -> Self {
    Self::new()
  }
}