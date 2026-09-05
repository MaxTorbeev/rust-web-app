use std::collections::{HashMap, HashSet};
use tokio::sync::Mutex;

use super::channel_state::ChannelState;
use crate::{ApplicationId, ChannelKey, ConnectionId, PresenceMutationOutcome};

/// Соединение в пределах приложения.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ConnectionKey {
  application_id: ApplicationId,
  connection_id: ConnectionId,
}

/// Сохранённый результат обработанной Presence-команды.
#[derive(Clone, Debug)]
struct PresenceOperationRecord {
  /// Хеш содержимого первоначальной команды.
  request_fingerprint: String,

  /// Результат, который необходимо вернуть при повторе команды.
  outcome: PresenceMutationOutcome,
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
