use crate::{ApplicationId, ApplicationSettings, AttachmentService, ChannelRouter, Connection, ConnectionCleanupError, InProcessChannelCommitDelivery, MemoryChannelStore, PresenceLedgerPolicy, PresenceService};
use support::fresh_uuid;
use auth::{TokenAccessIssuer, TokenAccessVerifier, VerifiedToken};
use std::sync::Arc;
use support::NodeInstance;
use support::timestamp::Timestamp;
use crate::connection::DisconnectConnectionCommand;

pub struct RealtimeApplication {
  pub id: ApplicationId,
  node_instance: NodeInstance,
  pub(crate) token_issuer: TokenAccessIssuer,
  pub token_verifier: TokenAccessVerifier,
  pub settings: ApplicationSettings,
  attachments: AttachmentService,
  router: Arc<ChannelRouter>,
  presence: PresenceService,
}

impl RealtimeApplication {
  /// Создаёт приложение в автономном режиме: локальное хранилище состояния
  /// каналов и синхронная доставка переходов в локальный `ChannelRouter`.
  ///
  /// Ограничения журнала Presence-операций берутся из настроек по умолчанию;
  /// изменение `settings` после создания на уже собранное хранилище не влияет.
  pub fn new(
    id: ApplicationId,
    node_instance: NodeInstance,
    token_issuer: TokenAccessIssuer,
    token_verifier: TokenAccessVerifier,
  ) -> Self {
    let settings = ApplicationSettings::default();
    let router = Arc::new(ChannelRouter::new());
    let store = Arc::new(MemoryChannelStore::with_ledger_policy(
      PresenceLedgerPolicy::from_settings(&settings),
    ));
    let delivery = Arc::new(InProcessChannelCommitDelivery::new(router.clone()));

    let attachments = AttachmentService::new(store.clone(), delivery.clone());
    let presence = PresenceService::new(store, delivery);

    Self::with_services(
      id,
      node_instance,
      token_issuer,
      token_verifier,
      router,
      attachments,
      presence,
    )
  }

  /// Создаёт приложение с внешне собранными хранилищем и доставкой.
  pub fn with_services(
    id: ApplicationId,
    node_instance: NodeInstance,
    token_issuer: TokenAccessIssuer,
    token_verifier: TokenAccessVerifier,
    router: Arc<ChannelRouter>,
    attachments: AttachmentService,
    presence: PresenceService,
  ) -> Self {
    Self {
      id,
      node_instance,
      token_issuer,
      token_verifier,
      settings: ApplicationSettings::default(),
      router,
      presence,
      attachments,
    }
  }

  pub fn router(&self) -> &ChannelRouter {
    self.router.as_ref()
  }

  pub fn attachments(&self) -> &AttachmentService {
    &self.attachments
  }

  pub fn presence(&self) -> &PresenceService {
    &self.presence
  }

  pub fn node_instance(&self) -> &NodeInstance {
    &self.node_instance
  }

  pub fn create_connection(&self, authorization: VerifiedToken) -> Connection {
    Connection::new(self, authorization)
  }

  /// Removes one connection from channel and presence state
  /// and broadcasts the resulting presence leave messages.
  pub async fn disconnect_connection(&self, connection: &Connection) -> Result<(), ConnectionCleanupError> {
    // Сначала исключаем закрывающееся соединение из локальной доставки.
    self.router().disconnect(&connection.id).await;

    self
      .attachments()
      .disconnect(DisconnectConnectionCommand {
        actor: connection.actor(),
        request_time: Timestamp::now(),
        event_id: fresh_uuid(),
      })
      .await?;

    Ok(())
  }
}
