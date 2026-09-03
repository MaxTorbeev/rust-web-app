use crate::{BootGeneration, NodeId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct NodeInstance {
  pub node_id: NodeId,
  pub boot_generation: BootGeneration,
}
