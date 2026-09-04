use crate::ConnectionId;
use support::NodeInstance;

#[derive(Debug, Clone)]
pub struct ConnectionActor {
  pub connection_id: ConnectionId,
  pub node_instance: NodeInstance,
}
