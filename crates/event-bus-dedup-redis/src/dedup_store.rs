use crate::config::RedisDedupStoreConfig;
use crate::protocol::{
  ClaimScriptOutcome, decode_claim_value, decode_transition_value, lease_value, redis_key,
  redis_ttl_milliseconds,
};
use crate::scripts;
use event_bus::{DedupClaim, DedupKey, DedupLease, DedupStore, DedupStoreError, DedupStoreFuture};
use redis_client::RedisClient;
use std::sync::Arc;
use std::time::Duration;
use support::fresh_uuid;

/// Redis-backed implementation of the event processing deduplication store.
///
/// A single Redis key represents one [`DedupKey`]. Its value is either a
/// temporary token-guarded lease or a completed marker, and every valid state
/// has a TTL.
pub struct RedisDedupStore {
  redis: Arc<RedisClient>,
  config: RedisDedupStoreConfig,
}

impl RedisDedupStore {
  /// Creates a store from an already connected Redis client and its namespace
  /// configuration.
  pub fn new(redis: Arc<RedisClient>, config: RedisDedupStoreConfig) -> Self {
    Self { redis, config }
  }
}

impl DedupStore for RedisDedupStore {
  fn claim<'a>(
    &'a self,
    key: &'a DedupKey,
    lease_ttl: Duration,
  ) -> DedupStoreFuture<'a, DedupClaim> {
    Box::pin(async move {
      let storage_key = redis_key(self.config.key_prefix(), key);
      let token = fresh_uuid();
      let lease_value = lease_value(token);
      let lease_ttl_ms = redis_ttl_milliseconds(lease_ttl)
        .map_err(DedupStoreError::backend)?
        .to_string();

      let keys = [storage_key.as_bytes()];
      let args = [lease_value.as_bytes(), lease_ttl_ms.as_bytes()];

      let value = self
        .redis
        .invoke_script(scripts::CLAIM, &keys, &args)
        .await
        .map_err(DedupStoreError::backend)?;

      match decode_claim_value(value).map_err(DedupStoreError::backend)? {
        ClaimScriptOutcome::Acquired => {
          Ok(DedupClaim::Acquired(DedupLease::new(key.clone(), token)))
        }
        ClaimScriptOutcome::Completed => Ok(DedupClaim::Completed),
        ClaimScriptOutcome::InProgress { retry_after } => {
          Ok(DedupClaim::InProgress { retry_after })
        }
      }
    })
  }

  fn complete<'a>(
    &'a self,
    lease: &'a DedupLease,
    completed_ttl: Duration,
  ) -> DedupStoreFuture<'a, ()> {
    Box::pin(async move {
      let storage_key = redis_key(self.config.key_prefix(), lease.key());
      let expected_lease = lease_value(lease.token());
      let completed_ttl_ms = redis_ttl_milliseconds(completed_ttl)
        .map_err(DedupStoreError::backend)?
        .to_string();

      let keys = [storage_key.as_bytes()];
      let args = [expected_lease.as_bytes(), completed_ttl_ms.as_bytes()];

      let value = self
        .redis
        .invoke_script(scripts::COMPLETE, &keys, &args)
        .await
        .map_err(DedupStoreError::backend)?;

      match decode_transition_value("complete", value).map_err(DedupStoreError::backend)? {
        true => Ok(()),
        false => Err(DedupStoreError::LeaseLost),
      }
    })
  }

  fn release<'a>(&'a self, lease: &'a DedupLease) -> DedupStoreFuture<'a, ()> {
    Box::pin(async move {
      let storage_key = redis_key(self.config.key_prefix(), lease.key());
      let expected_lease = lease_value(lease.token());

      let keys = [storage_key.as_bytes()];
      let args = [expected_lease.as_bytes()];

      let value = self
        .redis
        .invoke_script(scripts::RELEASE, &keys, &args)
        .await
        .map_err(DedupStoreError::backend)?;

      match decode_transition_value("release", value).map_err(DedupStoreError::backend)? {
        true => Ok(()),
        false => Err(DedupStoreError::LeaseLost),
      }
    })
  }
}
