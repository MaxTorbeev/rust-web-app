use support::NodeInstance;
use crate::{ConnectionId};

#[derive(Debug, Clone)]
pub struct ConnectionActor {
  pub connection_id: ConnectionId,
  pub owner: NodeInstance,
}