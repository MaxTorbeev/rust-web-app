use std::{future::Future, pin::Pin};

use crate::{ChannelCommitDeliveryError, CommittedChannelTransition};

pub type ChannelCommitDeliveryFuture<'a> =
  Pin<Box<dyn Future<Output = Result<(), ChannelCommitDeliveryError>> + Send + 'a>>;

/// Обрабатывает результат уже зафиксированного изменения состояния канала.
///
/// Метод вызывается только после того, как операция изменения состояния канала
/// была атомарно зафиксирована и хранилище вернуло [`CommittedChannelTransition`].
/// Ошибка последующей обработки не отменяет зафиксированную операцию: state и
/// журнал операций уже содержат результат, вызывающий может лишь сообщить об
/// ошибке наверх.
///
/// Разделение ответственности:
///
/// - Вызывающий (`AttachmentService`, `PresenceService`) передаёт каждый
///   зафиксированный переход **не более одного раза**. Воспроизведённый
///   результат повторной клиентской команды (`PresenceMutationReceipt::replayed`)
///   в delivery не попадает: его событие уже было передано при первой
///   обработке. Реализация вправе считать каждый вызов новым событием и не
///   обязана распознавать повтор одного `event_id`.
/// - Реализация отвечает за то, куда уходит событие. В автономном режиме —
///   в локальный projector для соединений этой ноды. В Redis-режиме событие
///   уже записано в outbox атомарно с state, поэтому реализация ничего не
///   публикует; fan-out начинает outbox publisher, а дедупликация повторов
///   транспорта выполняется потребителем по `event_id`.
///
/// Переход без события ([`CommittedChannelTransition::Unchanged`]) успешно
/// обрабатывается без дополнительных действий.
pub trait ChannelCommitDelivery: Send + Sync {
  fn after_commit<'a>(
    &'a self,
    transition: &'a CommittedChannelTransition,
  ) -> ChannelCommitDeliveryFuture<'a>;
}
