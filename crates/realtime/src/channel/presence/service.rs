use crate::channel::presence::command::PresenceBatchCommand;
use crate::{PresenceCommitDelivery, PresenceError, PresenceMutationOutcome, PresenceStore};
use std::sync::Arc;

pub struct PresenceService {
  store: Arc<dyn PresenceStore>,
  delivery: Arc<dyn PresenceCommitDelivery>,
}

impl PresenceService {
  pub fn new(store: Arc<dyn PresenceStore>, delivery: Arc<dyn PresenceCommitDelivery>) -> Self {
    Self { store, delivery }
  }

  pub async fn apply(
    &self,
    command: PresenceBatchCommand,
  ) -> Result<PresenceMutationOutcome, PresenceError> {
    let receipt = self.store.apply_presence(command).await?;

    if let PresenceMutationOutcome::Committed(transition) = &receipt.outcome {
      self.delivery.after_commit(transition).await?;
    }

    Ok(receipt.outcome)
  }
}
