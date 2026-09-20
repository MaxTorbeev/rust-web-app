use std::time::Duration;

use redis_client::RedisClient;
use redis_lease::{
  AcquireOutcome, LeaseKey, LeaseToken, RenewOutcome, decode_acquire, decode_renew, fence_key,
  lease_value, owner_value, redis_ttl_milliseconds,
};
use support::NodeInstance;

use super::super::{
  RedisKeys,
  protocol::{generation, node_lease_owner},
  scripts,
};
use super::{NodeClaimOutcome, NodeLeaseError};

/// Запуск ноды и token, полученный при захвате её lease.
#[derive(Debug)]
pub struct NodeLease {
  instance: NodeInstance,
  token: LeaseToken,
}

impl NodeLease {
  /// Захватывает node lease и регистрирует поколение одним Lua-вызовом.
  ///
  /// При ошибке запроса результат выполнения может остаться неизвестным.
  pub async fn claim(
    redis: &RedisClient,
    keys: &RedisKeys,
    instance: &NodeInstance,
    ttl: Duration,
  ) -> Result<NodeClaimOutcome, NodeLeaseError> {
    let lease_key = LeaseKey::new(keys.node_lease(&instance.node_id))?;
    let owner = node_lease_owner(instance)?;

    let counter_key = fence_key(&lease_key);
    let generations_key = keys.generations();
    let deadlines_key = keys.generation_deadlines();

    let owner_arg = owner_value(&owner);
    let ttl_arg = redis_ttl_milliseconds(ttl)?.to_string();
    let generation_id = generation(instance);
    let metadata = serde_json::to_string(instance)?;

    let script_keys = [
      lease_key.as_str().as_bytes(),
      counter_key.as_bytes(),
      generations_key.as_bytes(),
      deadlines_key.as_bytes(),
    ];

    let script_args = [
      owner_arg.as_bytes(),
      ttl_arg.as_bytes(),
      generation_id.as_bytes(),
      metadata.as_bytes(),
    ];

    let reply = redis
      .invoke_script(scripts::CLAIM_NODE, &script_keys, &script_args)
      .await?;

    match decode_acquire(reply, &lease_key, &owner)? {
      AcquireOutcome::Acquired { token } => Ok(NodeClaimOutcome::Acquired {
        lease: Self {
          instance: instance.clone(),
          token,
        },
      }),
      AcquireOutcome::Held { remaining } => Ok(NodeClaimOutcome::Held { remaining }),
    }
  }

  /// Продлевает lease и deadline поколения одним Lua-вызовом.
  ///
  /// `Lost` требует прекращения защищённой работы. При ошибке связи результат
  /// может быть неизвестен: работу приостанавливают до подтверждения владения
  /// повторным renew с тем же token. Метод не выполняет повторный claim.
  pub async fn renew(
    &self,
    redis: &RedisClient,
    keys: &RedisKeys,
    ttl: Duration,
  ) -> Result<RenewOutcome, NodeLeaseError> {
    let lease_key = keys.node_lease(&self.instance.node_id);
    if lease_key != self.token.key().as_str() {
      return Err(NodeLeaseError::NamespaceMismatch);
    }

    let deadlines_key = keys.generation_deadlines();
    let token_arg = lease_value(&self.token);
    let ttl_arg = redis_ttl_milliseconds(ttl)?.to_string();
    let generation_id = generation(&self.instance);

    let reply = redis
      .invoke_script(
        scripts::RENEW_NODE,
        &[lease_key.as_bytes(), deadlines_key.as_bytes()],
        &[
          token_arg.as_bytes(),
          ttl_arg.as_bytes(),
          generation_id.as_bytes(),
        ],
      )
      .await?;

    Ok(decode_renew(reply)?)
  }

  pub fn instance(&self) -> &NodeInstance {
    &self.instance
  }

  pub fn token(&self) -> &LeaseToken {
    &self.token
  }
}
