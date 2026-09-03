use crate::{timestamp::Timestamp, BootGeneration, NodeId};
use serde::{Deserialize, Serialize};

/// Конкретный запущенный экземпляр узла приложения.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct NodeInstance {
  pub node_id: NodeId,
  pub boot_generation: BootGeneration,
  /// Время запуска экземпляра приложения.
  pub started_at: Timestamp,
}

impl NodeInstance {
  pub fn new(node_id: NodeId, boot_generation: BootGeneration, started_at: Timestamp) -> Self {
    Self {
      node_id,
      boot_generation,
      started_at,
    }
  }
}
