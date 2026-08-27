use std::sync::Arc;
use std::time::Duration;
use crate::{DedupClaim, DedupKey, DedupStore, EventDispatcher, EventMessage, IncomingEventError, IncomingEventOutcome};

pub struct IncomingEventProcessor {
  dispatcher: Arc<EventDispatcher>,
  dedup_store: Arc<dyn DedupStore>,
  scope: String,
  lease_ttl: Duration,
  /// Сколько времени после успешной обработки мы помним event_id как завершённый.
  completed_record_ttl: Duration,
}

impl IncomingEventProcessor {
  pub fn new(
    dispatcher: Arc<EventDispatcher>,
    dedup_store: Arc<dyn DedupStore>,
    scope: impl Into<String>,
    lease_ttl: Duration,
  ) -> Self {
    Self {
      dispatcher,
      dedup_store,
      scope: scope.into(),
      lease_ttl,
      completed_record_ttl: Default::default(),
    }
  }

  pub async fn process(&self, message: &EventMessage) -> Result<IncomingEventOutcome, IncomingEventError> {
    let key = DedupKey::new(
      self.scope.clone(),
      message.event_id(),
    );

    let claim = self
      .dedup_store
      .claim(&key, self.lease_ttl)
      .await
      .map_err(|source| IncomingEventError::Claim { source })?;

    match claim {
      DedupClaim::Completed => {
        Ok(IncomingEventOutcome::Duplicate)
      }

      DedupClaim::InProgress { retry_after } => {
        Ok(IncomingEventOutcome::InProgress {
          retry_after,
        })
      }

      DedupClaim::Acquired(lease) => {
        if let Err(source) =
          self.dispatcher.dispatch(message).await
        {
          let release_error =
            self.dedup_store.release(&lease).await.err();

          return Err(IncomingEventError::Dispatch {
            source,
            release_error,
          });
        }

        self.dedup_store
          .complete(&lease, self.completed_record_ttl)
          .await
          .map_err(|source| {
            IncomingEventError::Complete { source }
          })?;

        Ok(IncomingEventOutcome::Applied)
      }
    }
  }
}