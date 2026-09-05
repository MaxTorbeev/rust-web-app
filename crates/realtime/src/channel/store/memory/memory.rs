use tokio::sync::Mutex;
use super::store_state::MemoryStoreState;

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
