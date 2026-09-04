use std::sync::Arc;
use crate::{CommittedTransition, PresenceAttachOutcome, PresenceCommitDelivery, PresenceError, PresenceMutationOutcome, PresenceStore};
use crate::channel::attachment::{AttachCommand, DetachCommand};
use crate::channel::presence::command::PresenceBatchCommand;
use crate::connection::DisconnectConnectionCommand;

pub struct PresenceService {
  store: Arc<dyn PresenceStore>,
  delivery: Arc<dyn PresenceCommitDelivery>,
}

impl PresenceService {

  pub fn new(
    store: Arc<dyn PresenceStore>,
    delivery: Arc<dyn PresenceCommitDelivery>,
  ) -> Self {
    Self { store, delivery }
  }


  /// Запрос на начало работы с каналом в текущей WebSocket-сессии.
  ///
  /// ATTACH сообщает серверу: «в этой сессии клиент начинает работать с таким-то каналом».
  /// Сервер сохраняет режимы, Presence/Occupancy-настройки
  /// и начинает маршрутизировать события канала.
  pub async fn attach(&self, command: AttachCommand) -> Result<PresenceAttachOutcome, PresenceError> {
    let outcome = self.store.attach_and_snapshot(command).await?;

    self
      .delivery
      .after_commit(&outcome.transition)
      .await?;

    Ok(outcome)
  }

  pub async fn apply(&self, command: PresenceBatchCommand) -> Result<PresenceMutationOutcome, PresenceError> {
    let receipt = self.store.apply_presence(command).await?;

    if let PresenceMutationOutcome::Committed(transition) = &receipt.outcome {
      self.delivery.after_commit(transition).await?;
    }

    Ok(receipt.outcome)
  }

  pub async fn detach(&self, command: DetachCommand) -> Result<CommittedTransition, PresenceError> {
    let transition = self.store.detach(command).await?;

    self.delivery.after_commit(&transition).await?;

    Ok(transition)
  }

  pub async fn disconnect(&self, command: DisconnectConnectionCommand) -> Result<Vec<CommittedTransition>, PresenceError> {
    let transitions = self.store.disconnect(command).await?;

    for transition in &transitions {
      self.delivery.after_commit(transition).await?;
    }

    Ok(transitions)
  }
}