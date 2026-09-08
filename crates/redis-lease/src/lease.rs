use std::sync::Arc;
use std::time::Duration;

use redis_client::RedisClient;

use crate::error::RedisLeaseError;
use crate::identity::{LeaseKey, LeaseOwner, LeaseToken};
use crate::outcome::{AcquireOutcome, ReleaseOutcome, RenewOutcome};
use crate::protocol::{
  decode_acquire, decode_release, decode_renew, fence_key, lease_value, owner_value,
  redis_ttl_milliseconds,
};
use crate::scripts;

/// Операции lease поверх [`RedisClient`].
///
/// Экземпляр не хранит состояния и не привязан к ключу или владельцу: один
/// `RedisLease` обслуживает node lease, publish lease и cleanup lock-и.
pub struct RedisLease {
  redis: Arc<RedisClient>,
}

impl RedisLease {
  pub fn new(redis: Arc<RedisClient>) -> Self {
    Self { redis }
  }

  /// Захватывает lease на `ttl` или подтверждает уже имеющийся.
  ///
  /// Возвращает token периода владения — его предъявляют [`Self::renew`],
  /// [`Self::release`] и проверка `holds_lease` в чужих скриптах. Fence внутри
  /// token монотонный: вызывающий проверяет его перед действиями, защищёнными
  /// lease (см. [`crate::Fence`]).
  pub async fn acquire(
    &self,
    key: &LeaseKey,
    owner: &LeaseOwner,
    ttl: Duration,
  ) -> Result<AcquireOutcome, RedisLeaseError> {
    let ttl_ms = redis_ttl_milliseconds(ttl)?.to_string();
    let owner_value = owner_value(owner);
    let fence_key = fence_key(key);

    let keys = [key.as_str().as_bytes(), fence_key.as_bytes()];
    let args = [owner_value.as_bytes(), ttl_ms.as_bytes()];

    let value = self
      .redis
      .invoke_script(scripts::ACQUIRE, &keys, &args)
      .await?;

    decode_acquire(value, key, owner)
  }

  /// Продлевает lease текущего периода на `ttl`.
  ///
  /// `Lost` означает, что lease истёк, освобождён или принадлежит другому
  /// периоду — в том числе новому периоду того же владельца. Вызывающий обязан
  /// остановить защищённую работу, а не пытаться захватить lease заново в том
  /// же цикле: его действия могли пересечься с новым владельцем.
  pub async fn renew(
    &self,
    token: &LeaseToken,
    ttl: Duration,
  ) -> Result<RenewOutcome, RedisLeaseError> {
    let ttl_ms = redis_ttl_milliseconds(ttl)?.to_string();
    let token_value = lease_value(token);

    let keys = [token.key().as_str().as_bytes()];
    let args = [token_value.as_bytes(), ttl_ms.as_bytes()];

    let value = self
      .redis
      .invoke_script(scripts::RENEW, &keys, &args)
      .await?;

    decode_renew(value)
  }

  /// Освобождает lease текущего периода. Чужой lease и lease другого периода
  /// того же владельца не трогаются: отложенный повтор release не удалит новый
  /// lease.
  pub async fn release(&self, token: &LeaseToken) -> Result<ReleaseOutcome, RedisLeaseError> {
    let token_value = lease_value(token);

    let keys = [token.key().as_str().as_bytes()];
    let args = [token_value.as_bytes()];

    let value = self
      .redis
      .invoke_script(scripts::RELEASE, &keys, &args)
      .await?;

    decode_release(value)
  }
}
