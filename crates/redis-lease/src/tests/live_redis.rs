//! Live-тесты против настоящего Redis.
//!
//! Запуск: `REDIS_LEASE_TEST_PORT=6379 cargo test -p redis-lease -- --ignored`.
//! Каждый тест использует собственное пространство ключей, поэтому тесты не
//! мешают друг другу и могут идти параллельно.

use std::sync::Arc;
use std::time::Duration;

use redis_client::{RedisClient, RedisConfig, ScriptValue};
use tokio::time::sleep;

use crate::protocol::fence_key;
use crate::{
  AcquireOutcome, Fence, LUA_HOLDS_LEASE, LeaseKey, LeaseOwner, LeaseToken, RedisLease,
  RedisLeaseError, ReleaseOutcome, RenewOutcome, lease_value,
};

const TTL: Duration = Duration::from_secs(30);
const SHORT_TTL: Duration = Duration::from_millis(150);

/// Скрипт «как в transition хранилища»: проверка владения первой строкой.
const HOLDS_LEASE_PROBE: &str =
  "\nif not holds_lease(KEYS[1], ARGV[1]) then return 0 end\nreturn 1";

struct Fixture {
  redis: Arc<RedisClient>,
  lease: RedisLease,
  prefix: String,
}

impl Fixture {
  async fn connect(test_name: &str) -> Self {
    let defaults = RedisConfig::default();
    let config = RedisConfig {
      host: std::env::var("REDIS_LEASE_TEST_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned()),
      port: std::env::var("REDIS_LEASE_TEST_PORT")
        .expect("set REDIS_LEASE_TEST_PORT to run ignored Redis lease tests"),
      username: std::env::var("REDIS_LEASE_TEST_USERNAME").ok(),
      password: std::env::var("REDIS_LEASE_TEST_PASSWORD").ok(),
      ..defaults
    };

    let redis = Arc::new(
      RedisClient::connect(&config)
        .await
        .expect("test Redis must be reachable"),
    );

    Self {
      lease: RedisLease::new(Arc::clone(&redis)),
      redis,
      prefix: format!("redis_lease_test.{test_name}.{}", uuid_like()),
    }
  }

  fn key(&self, name: &str) -> LeaseKey {
    LeaseKey::new(format!("{}.{name}", self.prefix)).unwrap()
  }

  /// Захват, который обязан пройти.
  async fn acquire(&self, key: &LeaseKey, owner: &LeaseOwner, ttl: Duration) -> LeaseToken {
    match self.lease.acquire(key, owner, ttl).await.unwrap() {
      AcquireOutcome::Acquired { token } => token,
      AcquireOutcome::Held { remaining } => {
        panic!("expected to hold the lease, held by other for {remaining:?}")
      }
    }
  }

  /// Текущее значение lease-ключа глазами Redis.
  async fn stored(&self, key: &LeaseKey) -> Option<String> {
    let value = self
      .redis
      .invoke_script(
        "return redis.call('GET', KEYS[1])",
        &[key.as_str().as_bytes()],
        &[],
      )
      .await
      .unwrap();

    match value {
      ScriptValue::Bytes(bytes) => Some(String::from_utf8(bytes).unwrap()),
      ScriptValue::Null => None,
      other => panic!("unexpected GET reply: {other:?}"),
    }
  }

  /// Записывает в lease-ключ произвольное значение, минуя протокол.
  async fn store_raw(&self, key: &LeaseKey, value: &str, ttl: Option<Duration>) {
    let script = match ttl {
      Some(_) => "redis.call('SET', KEYS[1], ARGV[1], 'PX', ARGV[2]); return 1",
      None => "redis.call('SET', KEYS[1], ARGV[1]); return 1",
    };
    let ttl_ms = ttl.map(|ttl| ttl.as_millis().to_string());
    let mut args = vec![value.as_bytes()];
    if let Some(ttl_ms) = &ttl_ms {
      args.push(ttl_ms.as_bytes());
    }

    self
      .redis
      .invoke_script(script, &[key.as_str().as_bytes()], &args)
      .await
      .unwrap();
  }

  /// `holds_lease` внутри чужого скрипта: 1 — token владеет ключом, 0 — нет.
  async fn holds_lease(&self, key: &LeaseKey, token_value: &str) -> i64 {
    let script = format!("{LUA_HOLDS_LEASE}{HOLDS_LEASE_PROBE}");
    let value = self
      .redis
      .invoke_script(
        &script,
        &[key.as_str().as_bytes()],
        &[token_value.as_bytes()],
      )
      .await
      .unwrap();

    match value {
      ScriptValue::Integer(value) => value,
      other => panic!("unexpected probe reply: {other:?}"),
    }
  }

  async fn cleanup(&self, key: &LeaseKey) {
    let fence_key = fence_key(key);
    let keys = [key.as_str().as_bytes(), fence_key.as_bytes()];
    self
      .redis
      .invoke_script(
        "redis.call('DEL', KEYS[1]); redis.call('DEL', KEYS[2]); return 1",
        &keys,
        &[],
      )
      .await
      .expect("cleanup must succeed");
  }
}

/// Уникальный суффикс без зависимости от uuid: время + счётчик процесса.
fn uuid_like() -> String {
  use std::sync::atomic::{AtomicU64, Ordering};
  static COUNTER: AtomicU64 = AtomicU64::new(0);
  let nanos = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_nanos();
  format!("{nanos}.{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

fn owner(name: &str) -> LeaseOwner {
  LeaseOwner::new(name).unwrap()
}

/// Token, которого Redis не выдавал: чужой владелец или чужой fence.
fn forged(key: &LeaseKey, owner: &LeaseOwner, fence: u64) -> LeaseToken {
  LeaseToken::new(key.clone(), owner.clone(), Fence::new(fence))
}

#[ignore = "requires a live Redis (REDIS_LEASE_TEST_PORT)"]
#[tokio::test]
async fn only_one_of_two_contenders_acquires() {
  let fx = Fixture::connect("contenders").await;
  let key = fx.key("lease");
  let (a, b) = (owner("node-a"), owner("node-b"));

  let token_a = fx.acquire(&key, &a, TTL).await;
  assert_eq!(token_a.key(), &key);
  assert_eq!(token_a.owner(), &a);

  let second = fx.lease.acquire(&key, &b, TTL).await.unwrap();
  let AcquireOutcome::Held { remaining } = second else {
    panic!("second contender must not acquire a held lease: {second:?}");
  };
  assert!(
    remaining > Duration::ZERO && remaining <= TTL,
    "{remaining:?}"
  );

  // Повторный acquire владельцем в непрерывном периоде — подтверждение с тем же
  // token, не новый период: потерянный ответ Redis не меняет идентичность.
  let again = fx.acquire(&key, &a, TTL).await;
  assert_eq!(again, token_a);

  fx.cleanup(&key).await;
}

#[ignore = "requires a live Redis (REDIS_LEASE_TEST_PORT)"]
#[tokio::test]
async fn stored_value_is_the_lease_value_of_the_token() {
  // Значение собирает Lua (`acquire.lua`), а `lease_value` — Rust. Расхождение
  // означало бы, что renew/release/holds_lease никогда не находят свой lease.
  let fx = Fixture::connect("stored_value").await;
  let key = fx.key("lease");
  let a = owner("node-a:293a2951-5ba0-482c-91c7-0a0c72a5ce4b");

  let token = fx.acquire(&key, &a, TTL).await;

  assert_eq!(
    fx.stored(&key).await.as_deref(),
    Some(lease_value(&token).as_str())
  );
  assert_eq!(
    lease_value(&token),
    format!(
      "lease:node-a:293a2951-5ba0-482c-91c7-0a0c72a5ce4b:{}",
      token.fence().get()
    )
  );

  fx.cleanup(&key).await;
}

#[ignore = "requires a live Redis (REDIS_LEASE_TEST_PORT)"]
#[tokio::test]
async fn renew_and_release_only_work_for_the_current_token() {
  let fx = Fixture::connect("current_token_only").await;
  let key = fx.key("lease");
  let (a, b) = (owner("node-a"), owner("node-b"));
  let token = fx.acquire(&key, &a, TTL).await;

  // Чужой владелец с угаданным fence и свой владелец с чужим fence — оба мимо.
  let other_owner = forged(&key, &b, token.fence().get());
  let other_fence = forged(&key, &a, token.fence().get() + 1);
  for stranger in [&other_owner, &other_fence] {
    assert_eq!(
      fx.lease.renew(stranger, TTL).await.unwrap(),
      RenewOutcome::Lost
    );
    assert_eq!(
      fx.lease.release(stranger).await.unwrap(),
      ReleaseOutcome::Lost
    );
  }
  // Чужие попытки ничего не изменили: владелец всё ещё продлевает.
  assert_eq!(
    fx.lease.renew(&token, TTL).await.unwrap(),
    RenewOutcome::Renewed
  );

  assert_eq!(
    fx.lease.release(&token).await.unwrap(),
    ReleaseOutcome::Released
  );
  // После release владелец уже никто: renew и повторный release — Lost.
  assert_eq!(
    fx.lease.renew(&token, TTL).await.unwrap(),
    RenewOutcome::Lost
  );
  assert_eq!(
    fx.lease.release(&token).await.unwrap(),
    ReleaseOutcome::Lost
  );

  fx.cleanup(&key).await;
}

#[ignore = "requires a live Redis (REDIS_LEASE_TEST_PORT)"]
#[tokio::test]
async fn stale_token_of_the_same_owner_does_not_touch_the_new_period() {
  // Сценарий бага: A захватил → освободил → захватил снова; отложенный повтор
  // старого release (и любая старая работа A) не должны задевать новый период.
  let fx = Fixture::connect("stale_same_owner").await;
  let key = fx.key("lease");
  let a = owner("node-a");

  let stale = fx.acquire(&key, &a, TTL).await;
  assert_eq!(
    fx.lease.release(&stale).await.unwrap(),
    ReleaseOutcome::Released
  );
  let current = fx.acquire(&key, &a, TTL).await;
  assert_ne!(current, stale, "a new period must have a new token");
  assert!(current.fence() > stale.fence());

  // Отложенный повтор release прошлого периода — Lost, новый lease на месте.
  assert_eq!(
    fx.lease.release(&stale).await.unwrap(),
    ReleaseOutcome::Lost
  );
  assert_eq!(
    fx.stored(&key).await.as_deref(),
    Some(lease_value(&current).as_str())
  );

  // Отложенный renew прошлого периода тоже не проходит.
  assert_eq!(
    fx.lease.renew(&stale, TTL).await.unwrap(),
    RenewOutcome::Lost
  );

  // Работа прошлого периода не проходит проверку владения; текущая — проходит.
  assert_eq!(fx.holds_lease(&key, &lease_value(&stale)).await, 0);
  assert_eq!(fx.holds_lease(&key, &lease_value(&current)).await, 1);

  assert_eq!(
    fx.lease.release(&current).await.unwrap(),
    ReleaseOutcome::Released
  );

  fx.cleanup(&key).await;
}

#[ignore = "requires a live Redis (REDIS_LEASE_TEST_PORT)"]
#[tokio::test]
async fn re_acquire_after_expiry_starts_a_new_period_for_the_same_owner() {
  // Нода потеряла связь, lease истёк, никто его не занял, нода вернулась и
  // захватила снова: её работа, начатая до разрыва, должна быть отвергнута.
  let fx = Fixture::connect("expiry_same_owner").await;
  let key = fx.key("lease");
  let a = owner("node-a");

  let before = fx.acquire(&key, &a, SHORT_TTL).await;
  sleep(SHORT_TTL + Duration::from_millis(100)).await;
  let after = fx.acquire(&key, &a, TTL).await;

  assert!(after.fence() > before.fence(), "{before:?} -> {after:?}");
  assert_eq!(
    fx.lease.renew(&before, TTL).await.unwrap(),
    RenewOutcome::Lost
  );
  assert_eq!(fx.holds_lease(&key, &lease_value(&before)).await, 0);
  assert_eq!(fx.holds_lease(&key, &lease_value(&after)).await, 1);

  fx.cleanup(&key).await;
}

#[ignore = "requires a live Redis (REDIS_LEASE_TEST_PORT)"]
#[tokio::test]
async fn expired_lease_is_taken_over_with_a_greater_fence() {
  let fx = Fixture::connect("expiry").await;
  let key = fx.key("lease");
  let (a, b) = (owner("node-a"), owner("node-b"));

  let token_a = fx.acquire(&key, &a, SHORT_TTL).await;
  sleep(SHORT_TTL + Duration::from_millis(100)).await;

  let token_b = fx.acquire(&key, &b, TTL).await;
  assert!(
    token_b.fence() > token_a.fence(),
    "new owner must get a strictly greater fence: {token_a:?} -> {token_b:?}"
  );

  // Старый владелец очнулся: продлить не может, захватить заново — тоже.
  assert_eq!(
    fx.lease.renew(&token_a, TTL).await.unwrap(),
    RenewOutcome::Lost
  );
  assert!(matches!(
    fx.lease.acquire(&key, &a, TTL).await.unwrap(),
    AcquireOutcome::Held { .. }
  ));

  fx.cleanup(&key).await;
}

#[ignore = "requires a live Redis (REDIS_LEASE_TEST_PORT)"]
#[tokio::test]
async fn fence_survives_release_and_keeps_growing() {
  let fx = Fixture::connect("fence_monotonic").await;
  let key = fx.key("lease");
  let a = owner("node-a");

  let mut previous = fx.acquire(&key, &a, TTL).await;
  for _ in 0..3 {
    fx.lease.release(&previous).await.unwrap();
    let next = fx.acquire(&key, &a, TTL).await;
    assert!(
      next.fence() > previous.fence(),
      "fence must grow across release/acquire: {previous:?} -> {next:?}"
    );
    previous = next;
  }

  fx.cleanup(&key).await;
}

#[ignore = "requires a live Redis (REDIS_LEASE_TEST_PORT)"]
#[tokio::test]
async fn renew_extends_the_ttl() {
  let fx = Fixture::connect("renew_extends").await;
  let key = fx.key("lease");
  let a = owner("node-a");

  let token = fx.acquire(&key, &a, SHORT_TTL).await;
  sleep(SHORT_TTL / 2).await;
  assert_eq!(
    fx.lease.renew(&token, TTL).await.unwrap(),
    RenewOutcome::Renewed
  );
  sleep(SHORT_TTL).await;

  // Без renew lease бы уже истёк; с renew владелец всё ещё держит его.
  assert_eq!(
    fx.lease.renew(&token, TTL).await.unwrap(),
    RenewOutcome::Renewed
  );

  fx.cleanup(&key).await;
}

#[ignore = "requires a live Redis (REDIS_LEASE_TEST_PORT)"]
#[tokio::test]
async fn lua_fragment_checks_ownership_inside_a_foreign_script() {
  // Так lease встраивается в transition хранилища: проверка владения — первой
  // строкой того же атомарного скрипта, что меняет состояние.
  let fx = Fixture::connect("fragment").await;
  let key = fx.key("lease");
  let (a, b) = (owner("node-a"), owner("node-b"));
  let token = fx.acquire(&key, &a, TTL).await;

  assert_eq!(fx.holds_lease(&key, &lease_value(&token)).await, 1);
  assert_eq!(
    fx.holds_lease(&key, &lease_value(&forged(&key, &b, token.fence().get())))
      .await,
    0
  );

  fx.cleanup(&key).await;
}

#[ignore = "requires a live Redis (REDIS_LEASE_TEST_PORT)"]
#[tokio::test]
async fn corrupt_lease_without_ttl_is_an_error_not_a_hold() {
  let fx = Fixture::connect("corrupt").await;
  let key = fx.key("lease");
  let (a, b) = (owner("node-a"), owner("node-b"));

  // Lease без TTL никогда не истечёт: это повреждённое состояние, и acquire
  // другим владельцем должен сообщить об ошибке, а не ждать вечно.
  fx.store_raw(&key, &lease_value(&forged(&key, &a, 1)), None)
    .await;

  let result = fx.lease.acquire(&key, &b, TTL).await;
  assert!(
    matches!(result, Err(RedisLeaseError::Redis(_))),
    "{result:?}"
  );

  fx.cleanup(&key).await;
}

#[ignore = "requires a live Redis (REDIS_LEASE_TEST_PORT)"]
#[tokio::test]
async fn unparseable_lease_value_is_an_error_not_a_hold() {
  // Значение без fence этому крейту не принадлежит: чужой ключ или повреждение.
  // Молча считать его «занято» значило бы скрыть ошибку конфигурации.
  let fx = Fixture::connect("unparseable").await;
  let key = fx.key("lease");
  let a = owner("node-a");

  fx.store_raw(&key, "lease:node-a", Some(TTL)).await;

  let result = fx.lease.acquire(&key, &a, TTL).await;
  assert!(
    matches!(result, Err(RedisLeaseError::Redis(_))),
    "{result:?}"
  );

  fx.cleanup(&key).await;
}
