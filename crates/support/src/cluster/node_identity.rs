use crate::{DeploymentSlot, NodeId};

/// Идентичность ноды для health и deployment controller.
///
/// `id` — стабильный `APP_NODE_ID`, по которому проверяются полнота и
/// уникальность набора нод, а оператор сопоставляет ноду с её durable
/// consumer-ом и owner lease. `slot` — deployment-группа из `DEPLOYMENT_SLOT`.
///
/// В отличие от `NodeInstance`, не содержит `boot_generation`: identity
/// описывает ноду, а не конкретный её запуск, и не участвует в событиях.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeIdentity {
  id: NodeId,
  slot: DeploymentSlot,
}

impl NodeIdentity {
  pub const fn new(id: NodeId, slot: DeploymentSlot) -> Self {
    Self { id, slot }
  }

  pub fn id(&self) -> &NodeId {
    &self.id
  }

  pub const fn slot(&self) -> DeploymentSlot {
    self.slot
  }
}
