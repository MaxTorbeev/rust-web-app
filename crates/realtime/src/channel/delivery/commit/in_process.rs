use std::sync::Arc;

use crate::{
  ChannelCommitDelivery, ChannelCommitDeliveryError, ChannelCommitDeliveryFuture, ChannelRouter,
  CommittedChannelTransition, PresenceMessage, ProtocolMessage,
};

/// Доставка зафиксированных переходов в пределах одного процесса.
///
/// Корректна только в автономном режиме, когда вся аудитория канала подключена
/// к этой ноде: событие уже зафиксировано хранилищем и здесь синхронно
/// проецируется в `PRESENCE`-кадры для соединений `ChannelRouter`, минуя event
/// bus и outbox. В кластере её место занимает outbox-вариант. Переход без изменений
/// участников не создаёт кадров. Повторная доставка того же события даёт тот же
/// кадр — дедупликация на стороне клиента возможна по `id` элементов.
pub struct InProcessChannelCommitDelivery {
  router: Arc<ChannelRouter>,
}

impl InProcessChannelCommitDelivery {
  pub fn new(router: Arc<ChannelRouter>) -> Self {
    Self { router }
  }
}

impl ChannelCommitDelivery for InProcessChannelCommitDelivery {
  fn after_commit<'a>(
    &'a self,
    transition: &'a CommittedChannelTransition,
  ) -> ChannelCommitDeliveryFuture<'a> {
    Box::pin(async move {
      let CommittedChannelTransition::Changed(event) = transition else {
        return Ok(());
      };

      let change = event.change();

      // TODO(occupancy): проецировать `change.occupancy` подписчикам Occupancy.
      if change.member_changes.is_empty() {
        return Ok(());
      }

      let presence = change
        .member_changes
        .iter()
        .map(PresenceMessage::from)
        .collect();

      self
        .router
        .broadcast(
          &change.channel.channel,
          ProtocolMessage::presence(&change.channel.channel, presence),
        )
        .await
        .map_err(ChannelCommitDeliveryError::LocalDelivery)?;

      Ok(())
    })
  }
}
