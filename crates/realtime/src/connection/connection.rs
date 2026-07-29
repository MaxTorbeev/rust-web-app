use auth::{UserIdentity, VerifiedToken};
use support::timestamp::Timestamp;
use crate::ConnectionId;

pub struct Connection {
  pub id: ConnectionId,
  pub authorization: VerifiedToken,
  pub connected_at: Timestamp
}

impl Connection {
  pub fn new(authorization: VerifiedToken) -> Self {
    Self {
      id: ConnectionId::generate(),
      authorization,
      connected_at: Timestamp::now()
    }
  }

  pub fn client_id(&self) -> Option<&str> {
    self.authorization.client_id.as_deref()
  }

  pub fn authorization(&self) -> &VerifiedToken {
    &self.authorization
  }
}