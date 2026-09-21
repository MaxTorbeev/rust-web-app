use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use redis_client::RedisClient;
use redis_lease::{LeaseToken, lease_value};
use support::NodeInstance;

use crate::{
  AttachCommand, AttachmentTracking, ChannelAttachOutcome, ChannelKey, ChannelStateStoreError,
  CommittedChannelTransition, ConnectionActor, DetachCommand, DisconnectConnectionCommand,
  PresenceBatchCommand, PresenceLedgerPolicy, PresenceMutationReceipt, PresenceSnapshot,
};

use super::RedisKeys;
use super::node::{NodeLease, NodeLeaseError};
use super::protocol::{generation, operation_serial, segment};
use super::request::{encode_disconnect_channels, encode_presence_items};
use super::response::{
  DisconnectReply, decode_attach, decode_disconnect, decode_expired_generations, decode_presence,
  decode_snapshot, decode_transition,
};
use super::scripts;

/// Redis-хранилище состояния каналов, обслуживаемое текущим запуском ноды.
pub struct RedisChannelStore {
  pub(super) redis: Arc<RedisClient>,
  pub(super) keys: RedisKeys,
  pub(super) node_lease: Arc<NodeLease>,
  ledger_policy: PresenceLedgerPolicy,
  available: AtomicBool,
}

impl RedisChannelStore {
  /// Создаёт адаптер для уже захваченного node lease.
  pub fn new(
    redis: Arc<RedisClient>,
    keys: RedisKeys,
    node_lease: Arc<NodeLease>,
    ledger_policy: PresenceLedgerPolicy,
  ) -> Result<Self, NodeLeaseError> {
    let expected_key = keys.node_lease(&node_lease.instance().node_id);

    if expected_key != node_lease.token().key().as_str() {
      return Err(NodeLeaseError::NamespaceMismatch);
    }

    Ok(Self {
      redis,
      keys,
      node_lease,
      available: AtomicBool::new(true),
      ledger_policy: PresenceLedgerPolicy {
        capacity: ledger_policy.capacity.max(1),
        ..ledger_policy
      },
    })
  }

  /// Сохраняет индивидуальный attachment и возвращает snapshot одним Lua-вызовом.
  ///
  /// Ошибка запроса или разбора ответа не означает, что изменения не сохранены.
  pub(super) async fn attach_and_snapshot(
    &self,
    command: AttachCommand,
  ) -> Result<ChannelAttachOutcome, ChannelStateStoreError> {
    self.validate_attach(&command)?;

    let instance = self.node_lease.instance();
    let application = &command.actor.application_id;
    let connection = &command.actor.connection_id;
    let channel = &command.channel;

    // Порядок KEYS и ARGV определён в docs/redis-attach-and-snapshot.md.
    let keys = [
      self.keys.node_lease(&instance.node_id),
      self.keys.generation_deadlines(),
      self.keys.channel_state(channel),
      self.keys.channel_attachments(channel),
      self.keys.channel_members(channel),
      self.keys.connection_state(application, connection),
      self.keys.connection_channels(application, connection),
      self.keys.connection_members(application, connection),
      self.keys.generation_connections(instance),
      self.keys.outbox(),
    ];
    let channel_json =
      serde_json::to_string(channel).map_err(|error| ChannelStateStoreError::Internal {
        message: format!("failed to serialize attach channel: {error}"),
      })?;
    let attachment_json = serde_json::to_string(&command.to_attachment()).map_err(|error| {
      ChannelStateStoreError::Internal {
        message: format!("failed to serialize attachment: {error}"),
      }
    })?;
    let args = [
      lease_value(self.node_lease.token()),
      generation(instance),
      segment(application.as_str()),
      segment(&channel.channel),
      segment(connection.as_str()),
      channel_json,
      attachment_json,
      command.event_id.to_string(),
    ];
    let script_keys = keys.each_ref().map(|key| key.as_bytes());
    let script_args = args.each_ref().map(|arg| arg.as_bytes());

    let reply = self
      .redis
      .invoke_script(scripts::ATTACH_AND_SNAPSHOT, &script_keys, &script_args)
      .await
      .map_err(|error| ChannelStateStoreError::Internal {
        message: format!("Redis attach_and_snapshot failed: {error}"),
      })?;

    decode_attach(&reply)
  }

  /// Применяет весь batch либо сохраняет доменный отказ; повтор возвращает прежний результат.
  ///
  /// При потере ответа повторяется исходная команда с тем же serial и fingerprint.
  pub(super) async fn apply_presence(
    &self,
    command: PresenceBatchCommand,
  ) -> Result<PresenceMutationReceipt, ChannelStateStoreError> {
    let actor = &command.actor.connection_actor;
    self.validate_actor(&command.channel, actor)?;
    let instance = self.node_lease.instance();
    let application = &actor.application_id;
    let connection = &actor.connection_id;
    let channel = &command.channel;

    // Порядок KEYS/ARGV: docs/redis-apply-presence.md.
    let keys = [
      self.keys.node_lease(&instance.node_id),
      self.keys.generation_deadlines(),
      self.keys.channel_state(channel),
      self.keys.channel_attachments(channel),
      self.keys.channel_members(channel),
      self.keys.connection_state(application, connection),
      self.keys.connection_channels(application, connection),
      self.keys.connection_members(application, connection),
      self.keys.generation_connections(instance),
      self.keys.outbox(),
      self
        .keys
        .connection_operation(application, connection, command.msg_serial),
      self
        .keys
        .connection_operation_order(application, connection),
    ];
    let serialization_error = |error: serde_json::Error| ChannelStateStoreError::Internal {
      message: format!("failed to serialize Presence command: {error}"),
    };
    let args = [
      lease_value(self.node_lease.token()),
      generation(instance),
      segment(application.as_str()),
      segment(&channel.channel),
      segment(connection.as_str()),
      serde_json::to_string(channel).map_err(serialization_error)?,
      connection.as_str().to_owned(),
      serde_json::to_string(&actor.node_instance).map_err(serialization_error)?,
      encode_presence_items(&command).map_err(serialization_error)?,
      operation_serial(command.msg_serial),
      command.request_fingerprint,
      command.event_id.to_string(),
      self.ledger_policy.capacity.to_string(),
    ];
    let script_keys = keys.each_ref().map(|key| key.as_bytes());
    let script_args = args.each_ref().map(|arg| arg.as_bytes());
    let reply = self
      .redis
      .invoke_script(scripts::APPLY_PRESENCE, &script_keys, &script_args)
      .await
      .map_err(|error| ChannelStateStoreError::Internal {
        message: format!("Redis apply_presence failed: {error}"),
      })?;
    decode_presence(&reply)
  }

  /// Удаляет attachment и его Presence members, сохраняя ledger соединения.
  /// Повтор без промежуточного attach возвращает Unchanged.
  pub(super) async fn detach(
    &self,
    command: DetachCommand,
  ) -> Result<CommittedChannelTransition, ChannelStateStoreError> {
    self.validate_actor(&command.channel, &command.actor)?;
    let instance = self.node_lease.instance();
    let application = &command.actor.application_id;
    let connection = &command.actor.connection_id;
    let channel = &command.channel;

    // Порядок KEYS/ARGV: docs/redis-detach.md.
    let keys = [
      self.keys.node_lease(&instance.node_id),
      self.keys.generation_deadlines(),
      self.keys.channel_state(channel),
      self.keys.channel_attachments(channel),
      self.keys.channel_members(channel),
      self.keys.connection_state(application, connection),
      self.keys.connection_channels(application, connection),
      self.keys.connection_members(application, connection),
      self.keys.generation_connections(instance),
      self.keys.outbox(),
    ];
    let serialization_error = |error: serde_json::Error| ChannelStateStoreError::Internal {
      message: format!("failed to serialize detach command: {error}"),
    };
    let args = [
      lease_value(self.node_lease.token()),
      generation(instance),
      segment(application.as_str()),
      segment(&channel.channel),
      segment(connection.as_str()),
      serde_json::to_string(channel).map_err(serialization_error)?,
      connection.as_str().to_owned(),
      serde_json::to_string(&command.actor.node_instance).map_err(serialization_error)?,
      command.event_id.to_string(),
    ];
    let script_keys = keys.each_ref().map(|key| key.as_bytes());
    let script_args = args.each_ref().map(|arg| arg.as_bytes());
    let reply = self
      .redis
      .invoke_script(scripts::DETACH, &script_keys, &script_args)
      .await
      .map_err(|error| ChannelStateStoreError::Internal {
        message: format!("Redis detach failed: {error}"),
      })?;
    decode_transition(&reply)
  }

  /// Удаляет соединение из всех каналов и закрывает ledger на время retention.
  /// Список каналов может потребовать повторной подготовки; commit всегда один.
  pub(super) async fn disconnect(
    &self,
    command: DisconnectConnectionCommand,
  ) -> Result<Vec<CommittedChannelTransition>, ChannelStateStoreError> {
    self.validate_generation(&command.actor)?;
    self.close_connection(command, None).await
  }

  /// Возвращает до limit кандидатов по возрастанию deadline; нулевой limit даёт пустой список.
  /// Перед очисткой reaper повторно проверяет leases и deadline поколения.
  pub(super) async fn expired_generations(
    &self,
    limit: u32,
  ) -> Result<Vec<NodeInstance>, ChannelStateStoreError> {
    let keys = [self.keys.generation_deadlines(), self.keys.generations()];
    let limit = limit.to_string();
    let script_keys = keys.each_ref().map(|key| key.as_bytes());
    let reply = self
      .redis
      .invoke_script(
        scripts::EXPIRED_GENERATIONS,
        &script_keys,
        &[limit.as_bytes()],
      )
      .await
      .map_err(|error| ChannelStateStoreError::Internal {
        message: format!("Redis expired_generations failed: {error}"),
      })?;
    decode_expired_generations(&reply)
  }

  /// Очищает одно соединение истёкшего поколения под cleanup lease текущей ноды.
  /// Actor указывает удаляемое соединение; lease store принадлежит reaper-у.
  pub(super) async fn reap_connection(
    &self,
    command: DisconnectConnectionCommand,
    cleanup_token: &LeaseToken,
  ) -> Result<Vec<CommittedChannelTransition>, ChannelStateStoreError> {
    if cleanup_token.key().as_str() != self.keys.cleanup_lease(&command.actor.node_instance)
      || cleanup_token.owner().as_str() != generation(self.node_lease.instance())
    {
      return Err(ChannelStateStoreError::InvalidRequest {
        message: "cleanup token must target the requested generation and belong to this reaper"
          .to_owned(),
      });
    }

    self.close_connection(command, Some(cleanup_token)).await
  }

  /// Общая подготовка каналов; Lua-вход выбирает проверки disconnect или reaper.
  async fn close_connection(
    &self,
    command: DisconnectConnectionCommand,
    cleanup_token: Option<&LeaseToken>,
  ) -> Result<Vec<CommittedChannelTransition>, ChannelStateStoreError> {
    let instance = &command.actor.node_instance;
    let application = &command.actor.application_id;
    let connection = &command.actor.connection_id;
    let mut keys = vec![
      self.keys.node_lease(&self.node_lease.instance().node_id),
      self.keys.generation_deadlines(),
      self.keys.connection_state(application, connection),
      self.keys.connection_channels(application, connection),
      self.keys.connection_members(application, connection),
      self.keys.generation_connections(instance),
      self.keys.outbox(),
      self
        .keys
        .connection_operation_order(application, connection),
      self.keys.connection_operations(application, connection),
    ];
    let serialization_error = |error: serde_json::Error| ChannelStateStoreError::Internal {
      message: format!("failed to serialize connection cleanup: {error}"),
    };
    let mut args = vec![
      lease_value(self.node_lease.token()),
      generation(instance),
      segment(application.as_str()),
      segment(connection.as_str()),
      connection.as_str().to_owned(),
      serde_json::to_string(&command.actor.node_instance).map_err(serialization_error)?,
      self.ledger_policy.retention.as_millis().to_string(),
      "[]".to_owned(),
    ];
    let (script, operation) = if let Some(token) = cleanup_token {
      keys.extend([
        token.key().as_str().to_owned(),
        self.keys.node_lease(&instance.node_id),
      ]);
      args.extend([generation(self.node_lease.instance()), lease_value(token)]);
      (scripts::REAP_CONNECTION, "reap_connection")
    } else {
      (scripts::DISCONNECT, "disconnect")
    };
    let base_key_count = keys.len();
    let mut channels = Vec::new();
    loop {
      // Фиксированные ключи соединения, затем state/attachments/members каждого канала.
      keys.truncate(base_key_count);
      for channel in &channels {
        keys.extend([
          self.keys.channel_state(channel),
          self.keys.channel_attachments(channel),
          self.keys.channel_members(channel),
        ]);
      }
      let script_keys: Vec<_> = keys.iter().map(|key| key.as_bytes()).collect();
      let script_args: Vec<_> = args.iter().map(|arg| arg.as_bytes()).collect();
      let reply = self
        .redis
        .invoke_script(script, &script_keys, &script_args)
        .await
        .map_err(|error| ChannelStateStoreError::Internal {
          message: format!("Redis {operation} failed: {error}"),
        })?;
      match decode_disconnect(&reply)? {
        DisconnectReply::Complete(transitions) => return Ok(transitions),
        DisconnectReply::Channels(names) => {
          channels = names
            .into_iter()
            .map(|name| ChannelKey::new(application.clone(), name))
            .collect();
          args[7] = encode_disconnect_channels(&command, &channels).map_err(serialization_error)?;
        }
      }
    }
  }

  /// Читает участников, версии и Occupancy counters одним Lua-вызовом.
  pub(super) async fn snapshot(
    &self,
    channel: ChannelKey,
  ) -> Result<PresenceSnapshot, ChannelStateStoreError> {
    let keys = [
      self.keys.channel_state(&channel),
      self.keys.channel_members(&channel),
      self.keys.channel_attachments(&channel),
      self.keys.channel_shards(&channel),
    ];
    let script_keys = keys.each_ref().map(|key| key.as_bytes());
    let reply = self
      .redis
      .invoke_script(scripts::SNAPSHOT, &script_keys, &[])
      .await
      .map_err(|error| ChannelStateStoreError::Internal {
        message: format!("Redis snapshot failed: {error}"),
      })?;

    decode_snapshot(&reply)
  }

  /// Проверяет параметры индивидуального attachment и принадлежность
  /// соединения запуску ноды, обслуживающему это хранилище.
  fn validate_attach(&self, command: &AttachCommand) -> Result<(), ChannelStateStoreError> {
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

    self.validate_actor(&command.channel, &command.actor)
  }

  fn validate_actor(
    &self,
    channel: &ChannelKey,
    actor: &ConnectionActor,
  ) -> Result<(), ChannelStateStoreError> {
    if !channel.belongs_to_application(&actor.application_id) {
      return Err(ChannelStateStoreError::InvalidRequest {
        message: "channel and connection belong to different applications".to_owned(),
      });
    }
    self.validate_generation(actor)
  }

  fn validate_generation(&self, actor: &ConnectionActor) -> Result<(), ChannelStateStoreError> {
    if !self.is_ready() {
      return Err(ChannelStateStoreError::Conflict {
        message: "Redis Presence runtime is stopped".into(),
      });
    }
    let actor_instance = &actor.node_instance;
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

  pub fn is_ready(&self) -> bool {
    self.available.load(Ordering::Acquire)
  }

  pub fn node_instance(&self) -> &NodeInstance {
    self.node_lease.instance()
  }

  pub(crate) fn stop(&self) {
    self.available.store(false, Ordering::Release);
  }
}
