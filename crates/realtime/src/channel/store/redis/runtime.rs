use super::{
  NodeClaimOutcome, NodeLease, RedisChannelStore, RedisKeys, protocol::node_lease_owner,
};
use crate::{ChannelStateStoreError, PresenceLedgerPolicy};
use event_bus::EventBus;
use redis_client::RedisClient;
use redis_lease::{AcquireOutcome, LeaseKey, RedisLease, RenewOutcome};
use std::{sync::Arc, time::Duration};
use support::NodeInstance;

type RuntimeError = Box<dyn std::error::Error + Send + Sync>;

/// Обязательные фоновые задачи Redis Presence; ошибка любой завершает весь runtime.
pub struct RedisPresenceRuntime {
  store: Arc<RedisChannelStore>,
}

impl Drop for RedisPresenceRuntime {
  fn drop(&mut self) {
    self.store.stop();
  }
}

impl RedisPresenceRuntime {
  pub const LEASE_TTL: Duration = Duration::from_secs(15);

  pub async fn claim(
    redis: Arc<RedisClient>,
    keys: RedisKeys,
    instance: &NodeInstance,
    ledger: PresenceLedgerPolicy,
  ) -> Result<Self, RuntimeError> {
    let lease = match NodeLease::claim(&redis, &keys, instance, Self::LEASE_TTL).await? {
      NodeClaimOutcome::Acquired { lease } => lease,
      NodeClaimOutcome::Held { .. } => return Err("node lease is held by another process".into()),
    };
    let store = Arc::new(RedisChannelStore::new(
      redis,
      keys,
      Arc::new(lease),
      ledger,
    )?);
    Ok(Self { store })
  }

  pub fn store(&self) -> Arc<RedisChannelStore> {
    self.store.clone()
  }

  /// Не возобновляет работу после неопределённого renewal: процесс должен перезапуститься.
  pub async fn run(self, bus: Arc<EventBus>) -> Result<(), RuntimeError> {
    tokio::select! {
      result = self.renew_node() => result,
      result = self.publish(&bus) => result,
      result = self.reap() => result,
    }
  }

  async fn renew_node(&self) -> Result<(), RuntimeError> {
    loop {
      if !self.store.is_ready() {
        return Err("Redis Presence projection stopped".into());
      }
      if self
        .store
        .node_lease
        .renew(&self.store.redis, &self.store.keys, Self::LEASE_TTL)
        .await?
        != RenewOutcome::Renewed
      {
        return Err("node lease lost".into());
      }
      tokio::time::sleep(Duration::from_secs(5)).await;
    }
  }

  async fn publish(&self, bus: &EventBus) -> Result<(), RuntimeError> {
    let leases = RedisLease::new(self.store.redis.clone());
    let key = LeaseKey::new(self.store.keys.publisher_lease())?;
    let owner = node_lease_owner(self.store.node_lease.instance())?;
    loop {
      match leases.acquire(&key, &owner, Self::LEASE_TTL).await? {
        AcquireOutcome::Held { .. } => tokio::time::sleep(Duration::from_millis(250)).await,
        AcquireOutcome::Acquired { token } => {
          let mut renew_at = tokio::time::Instant::now() + Duration::from_secs(5);
          loop {
            if tokio::time::Instant::now() >= renew_at {
              if leases.renew(&token, Self::LEASE_TTL).await? != RenewOutcome::Renewed {
                return Err("outbox publisher lease lost".into());
              }
              renew_at = tokio::time::Instant::now() + Duration::from_secs(5);
            }
            let count = tokio::time::timeout(
              Duration::from_secs(10),
              self.store.publish_outbox_batch(&token, bus),
            )
            .await??;
            if count == 0 {
              tokio::time::sleep(Duration::from_millis(250)).await;
            }
          }
        }
      }
    }
  }

  async fn reap(&self) -> Result<(), RuntimeError> {
    let leases = RedisLease::new(self.store.redis.clone());
    let owner = node_lease_owner(self.store.node_lease.instance())?;
    loop {
      for target in self.store.expired_generations(32).await? {
        let key = LeaseKey::new(self.store.keys.cleanup_lease(&target))?;
        let AcquireOutcome::Acquired { token } =
          leases.acquire(&key, &owner, Self::LEASE_TTL).await?
        else {
          continue;
        };
        // Один batch за проход: другая generation не голодает из-за большой ноды.
        let result = tokio::time::timeout(
          Duration::from_secs(10),
          self.store.reap_generation_batch(&target, &token, 32),
        )
        .await?;
        leases.release(&token).await?;
        match result {
          Ok(_) | Err(ChannelStateStoreError::Conflict { .. }) => {}
          Err(error) => return Err(error.into()),
        }
      }
      tokio::time::sleep(Duration::from_secs(1)).await;
    }
  }
}
