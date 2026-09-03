use crate::connection::ConnectionActor;

/// Актор, выполняющий presence операция и его полномочия
#[derive(Debug, Clone)]
pub struct PresenceActor {
  connection: ConnectionActor,
  /// All client ids allowed for this connection. Presence identity remains
  /// `(connection_id, client_id)`.
  pub authorized_client_ids: Vec<String>,
}