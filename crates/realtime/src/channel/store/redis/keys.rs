use support::app::{APP_NAMESPACE_SEPARATOR, AppNamespace, AppNamespaceError};
use support::{NodeId, NodeInstance};

use crate::{ApplicationId, ChannelKey, ConnectionId};

use super::protocol::{SCHEMA_VERSION, SUBSYSTEM, generation, segment};

/// Ключи Redis Presence v1 в namespace одного APP/APP_ENV.
///
/// Динамические идентификаторы кодируются независимо. Методы возвращают
/// полные ключи, не читают окружение и не обращаются к Redis.
/// Все lease-ключи выделены в `.lease.*`; суффикс `:fence` принадлежит
/// `redis-lease` и не используется для ключей данных этого адаптера.
#[derive(Clone, Debug)]
pub struct RedisKeys {
  namespace: AppNamespace,
}

impl RedisKeys {
  pub fn new(app: &str, environment: &str) -> Result<Self, AppNamespaceError> {
    Ok(Self {
      namespace: AppNamespace::try_new(app, environment, SUBSYSTEM, SCHEMA_VERSION)?,
    })
  }

  pub fn namespace(&self) -> &str {
    self.namespace.as_str()
  }

  /// HASH: Presence/Occupancy versions и шесть общих Occupancy counters.
  pub fn channel_state(&self, channel: &ChannelKey) -> String {
    self.channel_key(channel, "state")
  }

  /// HASH: connection → attachment.
  pub fn channel_attachments(&self, channel: &ChannelKey) -> String {
    self.channel_key(channel, "attachments")
  }

  /// HASH: (connection, client) → Presence member.
  pub fn channel_members(&self, channel: &ChannelKey) -> String {
    self.channel_key(channel, "members")
  }

  /// HASH: generation → абсолютный Occupancy shard с его version.
  pub fn channel_shards(&self, channel: &ChannelKey) -> String {
    self.channel_key(channel, "shards")
  }

  /// HASH: node generation, состояние ledger, highest serial и closed_at.
  pub fn connection_state(&self, app: &ApplicationId, connection: &ConnectionId) -> String {
    self.connection_key(app, connection, "state")
  }

  /// SET: каналы точных attachments соединения.
  pub fn connection_channels(&self, app: &ApplicationId, connection: &ConnectionId) -> String {
    self.connection_key(app, connection, "channels")
  }

  /// SET: (channel, client) для Presence members соединения.
  pub fn connection_members(&self, app: &ApplicationId, connection: &ConnectionId) -> String {
    self.connection_key(app, connection, "members")
  }

  /// HASH: msg_serial → fingerprint и сохранённый outcome.
  pub fn connection_operations(&self, app: &ApplicationId, connection: &ConnectionId) -> String {
    self.connection_key(app, connection, "operations")
  }

  /// ZSET: порядок msg_serial для ограниченного окна ledger; score всегда 0.
  pub fn connection_operation_order(
    &self,
    app: &ApplicationId,
    connection: &ConnectionId,
  ) -> String {
    self.connection_key(app, connection, "operation-order")
  }

  /// HASH: generation → метаданные запуска. Общий для всех applications.
  pub fn generations(&self) -> String {
    self.key(&["generations"])
  }

  /// ZSET: generation → deadline в миллисекундах Redis TIME.
  pub fn generation_deadlines(&self) -> String {
    self.key(&["generation-deadlines"])
  }

  /// SET: (application, connection) для cleanup точного состояния и ledger.
  pub fn generation_connections(&self, instance: &NodeInstance) -> String {
    self.key(&["generation", &generation(instance), "connections"])
  }

  /// SET: (application, channel) для cleanup aggregated Occupancy shards.
  pub fn generation_shards(&self, instance: &NodeInstance) -> String {
    self.key(&["generation", &generation(instance), "shards"])
  }

  /// STREAM: общая durable очередь canonical events.
  pub fn outbox(&self) -> String {
    self.key(&["outbox"])
  }

  /// ZSET: (application, channel) → ближайший Occupancy publish deadline.
  pub fn dirty_channels(&self) -> String {
    self.key(&["dirty-channels"])
  }

  /// HASH: (application, channel) → claimed Occupancy version и данные claim.
  pub fn occupancy_publications(&self) -> String {
    self.key(&["occupancy-publications"])
  }

  /// Lease стабилен по node ID: новый boot конкурирует за тот же ресурс.
  pub fn node_lease(&self, node: &NodeId) -> String {
    self.key(&["lease", "node", &segment(node.as_str())])
  }

  /// Lease единственного outbox publisher в namespace.
  pub fn publisher_lease(&self) -> String {
    self.key(&["lease", "publisher"])
  }

  /// Cleanup lock конкретной целевой generation; владелец — запуск reaper-а.
  pub fn cleanup_lease(&self, target: &NodeInstance) -> String {
    self.key(&["lease", "cleanup", &generation(target)])
  }

  fn channel_key(&self, channel: &ChannelKey, suffix: &str) -> String {
    self.key(&[
      "app",
      &segment(channel.application_id.as_str()),
      "channel",
      &segment(&channel.channel),
      suffix,
    ])
  }

  fn connection_key(&self, app: &ApplicationId, connection: &ConnectionId, suffix: &str) -> String {
    self.key(&[
      "app",
      &segment(app.as_str()),
      "connection",
      &segment(connection.as_str()),
      suffix,
    ])
  }

  fn key(&self, parts: &[&str]) -> String {
    format!(
      "{}{APP_NAMESPACE_SEPARATOR}{}",
      self.namespace(),
      parts.join(APP_NAMESPACE_SEPARATOR)
    )
  }
}
