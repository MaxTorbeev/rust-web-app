use super::ConnectionActor;
use crate::{ApplicationId, ConnectionId};

/// Соединение в пределах приложения.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ConnectionKey {
  pub(crate) application_id: ApplicationId,
  pub(crate) connection_id: ConnectionId,
}

impl From<&ConnectionActor> for ConnectionKey {
  fn from(actor: &ConnectionActor) -> Self {
    Self {
      application_id: actor.application_id.clone(),
      connection_id: actor.connection_id.clone(),
    }
  }
}
