use std::{future::Future, pin::Pin};

use crate::{ChannelCommitDeliveryError, CommittedChannelTransition};

pub type ChannelCommitDeliveryFuture<'a> =
  Pin<Box<dyn Future<Output = Result<(), ChannelCommitDeliveryError>> + Send + 'a>>;

/// Обрабатывает результат уже зафиксированного изменения состояния канала.
///
/// Метод вызывается только после того, как операция изменения состояния канала
/// была атомарно зафиксирована и хранилище вернуло [`CommittedChannelTransition`].
/// Ошибка последующей обработки не отменяет зафиксированную операцию.
///
/// В локальном режиме реализация передаёт созданное событие локальному
/// проектору. В Redis-режиме событие уже записано в outbox вместе с основным
/// состоянием, поэтому повторная публикация здесь не выполняется.
///
/// Переход без события успешно обрабатывается без дополнительных действий.
/// Повторная обработка одного перехода должна быть безопасной.
pub trait ChannelCommitDelivery: Send + Sync {
  fn after_commit<'a>(
    &'a self,
    transition: &'a CommittedChannelTransition,
  ) -> ChannelCommitDeliveryFuture<'a>;
}
