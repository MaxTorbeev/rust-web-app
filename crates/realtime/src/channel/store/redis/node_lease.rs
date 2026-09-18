use redis_lease::LeaseToken;
use support::NodeInstance;

/// Запуск ноды и token, полученный при захвате её lease.
#[derive(Debug)]
pub(super) struct NodeLease {
  instance: NodeInstance,
  token: LeaseToken,
}

impl NodeLease {
  pub(super) fn instance(&self) -> &NodeInstance {
    &self.instance
  }

  pub(super) fn token(&self) -> &LeaseToken {
    &self.token
  }
}