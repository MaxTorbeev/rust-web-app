use auth::UserIdentity;
use support::timestamp::Timestamp;
use crate::ConnectionId;

pub struct Connection {
  pub id: ConnectionId,
  pub user: UserIdentity,
  pub connected_at: Timestamp
}

impl Connection {
  pub fn new(user: UserIdentity) -> Self {
    Self {
      id: ConnectionId::generate(),
      user,
      connected_at: Timestamp::now()
    }
  }
}