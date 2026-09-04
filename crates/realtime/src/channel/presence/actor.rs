use crate::PresenceClientIdPolicy;
use crate::connection::ConnectionActor;

/// Актор, выполняющий presence операция и его полномочия
#[derive(Debug, Clone)]
pub struct PresenceActor {
  pub connection_actor: ConnectionActor,
  pub client_id_policy: PresenceClientIdPolicy,
}
