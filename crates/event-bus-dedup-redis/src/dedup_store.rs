use crate::config::RedisDedupStoreConfig;
use event_bus::{DedupClaim, DedupKey, DedupLease, DedupStore, DedupStoreFuture};
use redis_client::RedisClient;
use std::sync::Arc;
use std::time::Duration;

pub struct RedisDedupStore {
  redis: Arc<RedisClient>,
  config: Arc<RedisDedupStoreConfig>,
}

impl DedupStore for RedisDedupStore {
  fn claim<'a>(
    &'a self,
    key: &'a DedupKey,
    lease_ttl: Duration,
  ) -> DedupStoreFuture<'a, DedupClaim> {
    todo!()
  }

  fn complete<'a>(
    &'a self,
    lease: &'a DedupLease,
    completed_ttl: Duration,
  ) -> DedupStoreFuture<'a, ()> {
    todo!()
  }

  fn release<'a>(&'a self, lease: &'a DedupLease) -> DedupStoreFuture<'a, ()> {
    todo!()
  }
}
