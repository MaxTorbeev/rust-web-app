use crate::{ChannelCommitDelivery, ChannelCommitDeliveryFuture, CommittedChannelTransition};

/// Redis уже сохранил событие в outbox вместе с состоянием. Доставку начинает publisher.
pub struct OutboxChannelCommitDelivery;

impl ChannelCommitDelivery for OutboxChannelCommitDelivery {
  fn after_commit<'a>(
    &'a self,
    _: &'a CommittedChannelTransition,
  ) -> ChannelCommitDeliveryFuture<'a> {
    Box::pin(async { Ok(()) })
  }
}
