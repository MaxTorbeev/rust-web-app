use std::sync::Arc;

use crate::{
  ChannelCommitDelivery, ChannelCommitDeliveryFuture, ChannelRouter, CommittedChannelTransition,
};

/// Доставка зафиксированных переходов в пределах одного процесса.
///
/// Корректна только в автономном режиме, когда вся аудитория канала подключена
/// к этой ноде: событие уже зафиксировано хранилищем и здесь синхронно
/// проецируется в `PRESENCE`-кадры для соединений `ChannelRouter`, минуя event
/// bus и outbox. В кластере её место занимает outbox-вариант. Переход без
/// изменений участников не создаёт кадров.
///
/// Общий projector сохраняет revision каждого подписчика, пропускает повторы
/// и восстанавливает snapshot при пропуске ревизий.
pub struct InProcessChannelCommitDelivery {
  router: Arc<ChannelRouter>,
  store: Arc<dyn crate::PresenceStore>,
}

impl InProcessChannelCommitDelivery {
  pub fn new(router: Arc<ChannelRouter>, store: Arc<dyn crate::PresenceStore>) -> Self {
    Self { router, store }
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

      self
        .router
        .project_presence(event.change(), self.store.as_ref())
        .await?;

      Ok(())
    })
  }
}
