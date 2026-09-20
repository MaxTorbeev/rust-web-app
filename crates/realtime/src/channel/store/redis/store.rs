use std::sync::Arc;

use redis_client::RedisClient;

use crate::{AttachCommand, AttachmentTracking, ChannelStateStoreError};

use super::RedisKeys;
use super::node::{NodeLease, NodeLeaseError};

/// Redis-хранилище состояния каналов, обслуживаемое текущим запуском ноды.
pub struct RedisChannelStore {
  redis: Arc<RedisClient>,
  keys: RedisKeys,
  node_lease: Arc<NodeLease>,
}

impl RedisChannelStore {
  /// Создаёт адаптер для уже захваченного node lease.
  pub(super) fn new(
    redis: Arc<RedisClient>,
    keys: RedisKeys,
    node_lease: Arc<NodeLease>,
  ) -> Result<Self, NodeLeaseError> {
    let expected_key = keys.node_lease(
      &node_lease.instance().node_id,
    );

    if expected_key != node_lease.token().key().as_str() {
      return Err(NodeLeaseError::NamespaceMismatch);
    }

    Ok(Self {
      redis,
      keys,
      node_lease,
    })
  }

  /// Проверяет параметры индивидуального attachment и принадлежность
  /// соединения запуску ноды, обслуживающему это хранилище.
  fn validate_attach(
    &self,
    command: &AttachCommand,
  ) -> Result<(), ChannelStateStoreError> {
    if command.accounting != AttachmentTracking::Individual {
      return Err(ChannelStateStoreError::InvalidRequest {
        message: "individual attachment accounting is required".to_owned(),
      });
    }

    if command.effective_modes.is_empty() {
      return Err(ChannelStateStoreError::InvalidRequest {
        message: "attachment requires at least one effective mode".to_owned(),
      });
    }

    if !command.channel.belongs_to_application(&command.actor.application_id) {
      return Err(ChannelStateStoreError::InvalidRequest {
        message: "channel and connection belong to different applications".to_owned(),
      });
    }

    let actor_instance = &command.actor.node_instance;
    let store_instance = self.node_lease.instance();

    if actor_instance.node_id != store_instance.node_id
      || actor_instance.boot_generation != store_instance.boot_generation
    {
      return Err(ChannelStateStoreError::InvalidRequest {
        message: "connection belongs to another node generation".to_owned(),
      });
    }

    Ok(())
  }
}
