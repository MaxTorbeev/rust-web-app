use crate::protocol::redis_key;
use crate::{RedisDedupStore, RedisDedupStoreConfig};
use event_bus::{
  DedupClaim, DedupKey, DedupLease, DedupStore, DedupStoreError, EVENT_BUS_NAMESPACE_VERSION,
  EVENT_BUS_SUBSYSTEM,
};
use redis_client::{RedisClient, RedisConfig, ScriptValue};
use std::sync::Arc;
use std::time::Duration;
use support::app::AppNamespace;
use tokio::time::{sleep, timeout};
use uuid::Uuid;

const DELETE_SCRIPT: &str = "return redis.call('DEL', KEYS[1])";
const PTTL_SCRIPT: &str = "return redis.call('PTTL', KEYS[1])";
const SET_PERSISTENT_SCRIPT: &str = "redis.call('SET', KEYS[1], ARGV[1]); return 1";
const SET_WITH_TTL_SCRIPT: &str = "redis.call('SET', KEYS[1], ARGV[1], 'PX', ARGV[2]); return 1";
const LIVE_TEST_TIMEOUT: Duration = Duration::from_secs(2);
const ORDINARY_TTL: Duration = Duration::from_secs(30);

struct Fixture {
  redis: Arc<RedisClient>,
  store: RedisDedupStore,
  config: RedisDedupStoreConfig,
}

impl Fixture {
  async fn connect() -> Self {
    let defaults = RedisConfig::default();
    let config = RedisConfig {
      host: std::env::var("REDIS_DEDUP_TEST_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned()),
      port: std::env::var("REDIS_DEDUP_TEST_PORT")
        .expect("set REDIS_DEDUP_TEST_PORT to run ignored Redis contract tests"),
      username: std::env::var("REDIS_DEDUP_TEST_USERNAME").ok(),
      password: std::env::var("REDIS_DEDUP_TEST_PASSWORD").ok(),
      ..defaults
    };

    let redis = Arc::new(
      RedisClient::connect(&config)
        .await
        .expect("test Redis must be reachable"),
    );
    let namespace = AppNamespace::try_new(
      "dedup_contract_test",
      "local",
      EVENT_BUS_SUBSYSTEM,
      EVENT_BUS_NAMESPACE_VERSION,
    )
    .expect("test namespace must be valid");
    let config = RedisDedupStoreConfig::new(&namespace);
    let store = RedisDedupStore::new(Arc::clone(&redis), config.clone());

    Self {
      redis,
      store,
      config,
    }
  }

  fn key(&self) -> DedupKey {
    DedupKey::new(format!("scope-{}", Uuid::new_v4()), Uuid::new_v4())
  }

  async fn delete(&self, key: &DedupKey) {
    let key = redis_key(self.config.key_prefix(), key);
    let keys = [key.as_bytes()];

    self
      .redis
      .invoke_script(DELETE_SCRIPT, &keys, &[])
      .await
      .expect("owned test key must be removable");
  }

  async fn pttl(&self, key: &DedupKey) -> Duration {
    let key = redis_key(self.config.key_prefix(), key);
    let keys = [key.as_bytes()];
    let value = self
      .redis
      .invoke_script(PTTL_SCRIPT, &keys, &[])
      .await
      .expect("test key TTL must be readable");

    match value {
      ScriptValue::Integer(milliseconds) if milliseconds >= 0 => {
        Duration::from_millis(milliseconds as u64)
      }
      value => panic!("expected a non-negative PTTL, got {value:?}"),
    }
  }

  async fn seed_with_ttl(&self, key: &DedupKey, value: &str, ttl: Duration) {
    let key = redis_key(self.config.key_prefix(), key);
    let ttl_ms = ttl.as_millis().to_string();
    let keys = [key.as_bytes()];
    let args = [value.as_bytes(), ttl_ms.as_bytes()];

    self
      .redis
      .invoke_script(SET_WITH_TTL_SCRIPT, &keys, &args)
      .await
      .expect("test state with TTL must be writable");
  }

  async fn seed_persistent(&self, key: &DedupKey, value: &str) {
    let key = redis_key(self.config.key_prefix(), key);
    let keys = [key.as_bytes()];
    let args = [value.as_bytes()];

    self
      .redis
      .invoke_script(SET_PERSISTENT_SCRIPT, &keys, &args)
      .await
      .expect("persistent test state must be writable");
  }
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires REDIS_DEDUP_TEST_PORT"]
async fn claim_and_complete_follow_the_token_guarded_state_machine() {
  let fixture = Fixture::connect().await;
  let key = fixture.key();
  let lease_ttl = ORDINARY_TTL;

  let (first, second) = tokio::join!(
    fixture.store.claim(&key, lease_ttl),
    fixture.store.claim(&key, lease_ttl),
  );

  let owner = match (first.unwrap(), second.unwrap()) {
    (DedupClaim::Acquired(lease), DedupClaim::InProgress { retry_after })
    | (DedupClaim::InProgress { retry_after }, DedupClaim::Acquired(lease)) => {
      assert!(retry_after <= lease_ttl);
      lease
    }
    outcomes => panic!("expected one acquired and one in-progress claim, got {outcomes:?}"),
  };

  let forged = DedupLease::new(key.clone(), Uuid::new_v4());
  assert!(matches!(
    fixture.store.complete(&forged, ORDINARY_TTL).await,
    Err(DedupStoreError::LeaseLost)
  ));

  assert!(matches!(
    fixture.store.claim(&key, lease_ttl).await.unwrap(),
    DedupClaim::InProgress { .. }
  ));

  fixture.store.complete(&owner, ORDINARY_TTL).await.unwrap();

  assert_eq!(
    fixture.store.claim(&key, lease_ttl).await.unwrap(),
    DedupClaim::Completed
  );
  assert!(matches!(
    fixture.store.release(&owner).await,
    Err(DedupStoreError::LeaseLost)
  ));
  assert_eq!(
    fixture.store.claim(&key, lease_ttl).await.unwrap(),
    DedupClaim::Completed
  );

  fixture.delete(&key).await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires REDIS_DEDUP_TEST_PORT"]
async fn release_requires_the_owner_token_and_allows_immediate_reclaim() {
  let fixture = Fixture::connect().await;
  let key = fixture.key();
  let owner = acquired(fixture.store.claim(&key, ORDINARY_TTL).await.unwrap());
  let forged = DedupLease::new(key.clone(), Uuid::new_v4());

  assert!(matches!(
    fixture.store.release(&forged).await,
    Err(DedupStoreError::LeaseLost)
  ));
  assert!(matches!(
    fixture.store.claim(&key, ORDINARY_TTL).await.unwrap(),
    DedupClaim::InProgress { .. }
  ));

  fixture.store.release(&owner).await.unwrap();

  let next = acquired(fixture.store.claim(&key, ORDINARY_TTL).await.unwrap());
  assert_ne!(next.token(), owner.token());

  fixture.store.release(&next).await.unwrap();
  fixture.delete(&key).await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires REDIS_DEDUP_TEST_PORT"]
async fn expired_owner_cannot_mutate_a_new_lease() {
  let fixture = Fixture::connect().await;
  let key = fixture.key();
  let expired = acquired(
    fixture
      .store
      .claim(&key, Duration::from_millis(20))
      .await
      .unwrap(),
  );

  let replacement = acquire_before_deadline(&fixture.store, &key).await;
  assert_ne!(replacement.token(), expired.token());

  assert!(matches!(
    fixture.store.complete(&expired, ORDINARY_TTL).await,
    Err(DedupStoreError::LeaseLost)
  ));
  assert!(matches!(
    fixture.store.release(&expired).await,
    Err(DedupStoreError::LeaseLost)
  ));
  assert!(matches!(
    fixture.store.claim(&key, ORDINARY_TTL).await.unwrap(),
    DedupClaim::InProgress { .. }
  ));

  fixture.store.release(&replacement).await.unwrap();
  fixture.delete(&key).await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires REDIS_DEDUP_TEST_PORT"]
async fn repeated_claim_does_not_refresh_lease_ttl() {
  let fixture = Fixture::connect().await;
  let key = fixture.key();
  let owner = acquired(fixture.store.claim(&key, ORDINARY_TTL).await.unwrap());
  let ttl_before = fixture.pttl(&key).await;

  sleep(Duration::from_millis(100)).await;

  assert!(matches!(
    fixture.store.claim(&key, ORDINARY_TTL).await.unwrap(),
    DedupClaim::InProgress { .. }
  ));
  let ttl_after = fixture.pttl(&key).await;

  assert!(
    ttl_after < ttl_before,
    "claim refreshed lease TTL from {ttl_before:?} to {ttl_after:?}"
  );

  fixture.store.release(&owner).await.unwrap();
  fixture.delete(&key).await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires REDIS_DEDUP_TEST_PORT"]
async fn completed_marker_is_not_refreshed_and_eventually_expires() {
  let fixture = Fixture::connect().await;
  let key = fixture.key();
  let owner = acquired(fixture.store.claim(&key, ORDINARY_TTL).await.unwrap());
  let completed_ttl = Duration::from_secs(1);

  fixture.store.complete(&owner, completed_ttl).await.unwrap();
  let ttl_before = fixture.pttl(&key).await;

  sleep(Duration::from_millis(100)).await;

  assert_eq!(
    fixture.store.claim(&key, ORDINARY_TTL).await.unwrap(),
    DedupClaim::Completed
  );
  let ttl_after = fixture.pttl(&key).await;
  assert!(
    ttl_after < ttl_before,
    "claim refreshed completed TTL from {ttl_before:?} to {ttl_after:?}"
  );

  let next = acquire_after_completed_expiry(&fixture.store, &key).await;
  assert_ne!(next.token(), owner.token());

  fixture.store.release(&next).await.unwrap();
  fixture.delete(&key).await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires REDIS_DEDUP_TEST_PORT"]
async fn corrupted_state_is_reported_as_backend_error() {
  let fixture = Fixture::connect().await;

  let unknown_state = fixture.key();
  fixture
    .seed_with_ttl(&unknown_state, "unknown", ORDINARY_TTL)
    .await;
  let unknown_result = fixture.store.claim(&unknown_state, ORDINARY_TTL).await;
  fixture.delete(&unknown_state).await;
  assert!(matches!(
    unknown_result,
    Err(DedupStoreError::Backend { .. })
  ));

  for value in ["completed", "lease:persistent-test-token"] {
    let persistent_state = fixture.key();
    fixture.seed_persistent(&persistent_state, value).await;
    let result = fixture.store.claim(&persistent_state, ORDINARY_TTL).await;
    fixture.delete(&persistent_state).await;

    assert!(matches!(result, Err(DedupStoreError::Backend { .. })));
  }
}

fn acquired(claim: DedupClaim) -> DedupLease {
  match claim {
    DedupClaim::Acquired(lease) => lease,
    claim => panic!("expected acquired claim, got {claim:?}"),
  }
}

async fn acquire_before_deadline(store: &RedisDedupStore, key: &DedupKey) -> DedupLease {
  timeout(LIVE_TEST_TIMEOUT, async {
    loop {
      match store.claim(key, ORDINARY_TTL).await.unwrap() {
        DedupClaim::Acquired(lease) => return lease,
        DedupClaim::InProgress { retry_after } => {
          sleep(retry_after.max(Duration::from_millis(1))).await;
        }
        DedupClaim::Completed => panic!("expired lease unexpectedly became completed"),
      }
    }
  })
  .await
  .expect("lease did not expire in time")
}

async fn acquire_after_completed_expiry(store: &RedisDedupStore, key: &DedupKey) -> DedupLease {
  timeout(LIVE_TEST_TIMEOUT, async {
    loop {
      match store.claim(key, ORDINARY_TTL).await.unwrap() {
        DedupClaim::Acquired(lease) => return lease,
        DedupClaim::Completed => sleep(Duration::from_millis(10)).await,
        DedupClaim::InProgress { .. } => {
          panic!("completed marker unexpectedly became an in-progress lease")
        }
      }
    }
  })
  .await
  .expect("completed marker did not expire in time")
}
