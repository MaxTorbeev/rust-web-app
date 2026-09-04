use std::sync::Arc;

use crate::channel::attachment::{AttachCommand, AttachmentError, DetachCommand};
use crate::connection::DisconnectConnectionCommand;
use crate::{CommittedTransition, PresenceAttachOutcome, PresenceCommitDelivery, PresenceStore};

/// Управляет жизненным циклом соединения с каналами.
///
/// Сервис фиксирует `ATTACH`, `DETACH` и отключение соединения
/// в хранилище, после чего передаёт созданные переходы в доставку.
///
/// Локальная регистрация соединения в [`crate::ChannelRouter`] выполняется
/// уровнем приложения или транспорта и не входит в ответственность сервиса.
pub struct AttachmentService {
  store: Arc<dyn PresenceStore>,
  delivery: Arc<dyn PresenceCommitDelivery>,
}

impl AttachmentService {
  pub fn new(store: Arc<dyn PresenceStore>, delivery: Arc<dyn PresenceCommitDelivery>) -> Self {
    Self { store, delivery }
  }

  /// Фиксирует начало работы соединения с каналом и возвращает
  /// состояние канала, необходимое для завершения `ATTACH`.
  pub async fn attach(
    &self,
    command: AttachCommand,
  ) -> Result<PresenceAttachOutcome, AttachmentError> {
    let outcome = self.store.attach_and_snapshot(command).await?;

    self.delivery.after_commit(&outcome.transition).await?;

    Ok(outcome)
  }

  /// Удаляет attachment соединения из одного канала.
  ///
  /// Если соединение было участником Presence, хранилище также фиксирует
  /// его выход и возвращает соответствующий переход.
  pub async fn detach(
    &self,
    command: DetachCommand,
  ) -> Result<CommittedTransition, AttachmentError> {
    let transition = self.store.detach(command).await?;

    self.delivery.after_commit(&transition).await?;

    Ok(transition)
  }

  /// Удаляет attachment-ы соединения из всех каналов.
  ///
  /// Для каждого затронутого канала обрабатывает переход, созданный
  /// хранилищем. Отсутствие Presence member не является ошибкой.
  pub async fn disconnect(
    &self,
    command: DisconnectConnectionCommand,
  ) -> Result<Vec<CommittedTransition>, AttachmentError> {
    let transitions = self.store.disconnect(command).await?;

    for transition in &transitions {
      self.delivery.after_commit(transition).await?;
    }

    Ok(transitions)
  }
}
