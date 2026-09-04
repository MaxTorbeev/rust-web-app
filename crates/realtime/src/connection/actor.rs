use crate::{ApplicationId, ConnectionId};
use support::NodeInstance;

#[derive(Debug, Clone)]
pub struct ConnectionActor {
  pub application_id: ApplicationId,
  pub connection_id: ConnectionId,
  pub node_instance: NodeInstance,
}
