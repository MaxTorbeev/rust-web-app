use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use event_bus::{
  DedupClaim, DedupKey, DedupLease, DedupStore, DedupStoreError, DedupStoreFuture, Event,
  EventDispatcher, EventMessage, HandlerError, IncomingEventError, IncomingEventOutcome,
  IncomingEventProcessor, IncomingEventProcessorConfig, IncomingEventProcessorConfigError,
  ProcessingErrorClass,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
struct TestEvent;

impl Event for TestEvent {
  const NAME: &'static str = "test.incoming_event";
}

#[derive(Default)]
struct RecordingDedupStore {
  completed_record_ttl: Mutex<Option<Duration>>,
  released_token: Mutex<Option<Uuid>>,
  release_fails: bool,
}

impl DedupStore for RecordingDedupStore {
  fn claim<'a>(
    &'a self,
    key: &'a DedupKey,
    _lease_ttl: Duration,
  ) -> DedupStoreFuture<'a, DedupClaim> {
    let lease = DedupLease::new(key.clone(), Uuid::new_v4());

    Box::pin(async move { Ok(DedupClaim::Acquired(lease)) })
  }

  fn complete<'a>(
    &'a self,
    _lease: &'a DedupLease,
    completed_record_ttl: Duration,
  ) -> DedupStoreFuture<'a, ()> {
    Box::pin(async move {
      *self.completed_record_ttl.lock().unwrap() = Some(completed_record_ttl);

      Ok(())
    })
  }

  fn release<'a>(&'a self, lease: &'a DedupLease) -> DedupStoreFuture<'a, ()> {
    Box::pin(async move {
      *self.released_token.lock().unwrap() = Some(lease.token());

      if self.release_fails {
        Err(DedupStoreError::LeaseLost)
      } else {
        Ok(())
      }
    })
  }
}

struct HandlerDropSignal(Arc<AtomicBool>);

impl Drop for HandlerDropSignal {
  fn drop(&mut self) {
    self.0.store(true, Ordering::SeqCst);
  }
}

#[test]
fn accepts_valid_configuration() {
  let config = IncomingEventProcessorConfig::try_new(
    "realtime-node-1",
    Duration::from_secs(20),
    Duration::from_secs(30),
    Duration::from_secs(86_400),
  )
  .unwrap();

  assert_eq!(config.scope(), "realtime-node-1");
  assert_eq!(config.processing_timeout(), Duration::from_secs(20));
  assert_eq!(config.lease_ttl(), Duration::from_secs(30));
  assert_eq!(config.completed_record_ttl(), Duration::from_secs(86_400));
}

#[test]
fn rejects_empty_scope() {
  let result = IncomingEventProcessorConfig::try_new(
    "",
    Duration::from_secs(20),
    Duration::from_secs(30),
    Duration::from_secs(86_400),
  );

  assert!(matches!(
    result,
    Err(IncomingEventProcessorConfigError::EmptyScope)
  ));
}

#[test]
fn rejects_zero_processing_timeout() {
  let result = IncomingEventProcessorConfig::try_new(
    "realtime-node-1",
    Duration::ZERO,
    Duration::from_secs(30),
    Duration::from_secs(86_400),
  );

  assert!(matches!(
    result,
    Err(IncomingEventProcessorConfigError::ZeroProcessingTimeout)
  ));
}

#[test]
fn rejects_zero_lease_ttl() {
  let result = IncomingEventProcessorConfig::try_new(
    "realtime-node-1",
    Duration::from_secs(20),
    Duration::ZERO,
    Duration::from_secs(86_400),
  );

  assert!(matches!(
    result,
    Err(IncomingEventProcessorConfigError::ZeroLeaseTtl)
  ));
}

#[test]
fn rejects_zero_completed_record_ttl() {
  let result = IncomingEventProcessorConfig::try_new(
    "realtime-node-1",
    Duration::from_secs(20),
    Duration::from_secs(30),
    Duration::ZERO,
  );

  assert!(matches!(
    result,
    Err(IncomingEventProcessorConfigError::ZeroCompletedRecordTtl)
  ));
}

#[test]
fn rejects_processing_timeout_not_less_than_lease_ttl() {
  for processing_timeout in [Duration::from_secs(30), Duration::from_secs(31)] {
    let result = IncomingEventProcessorConfig::try_new(
      "realtime-node-1",
      processing_timeout,
      Duration::from_secs(30),
      Duration::from_secs(86_400),
    );

    assert!(matches!(
      result,
      Err(IncomingEventProcessorConfigError::ProcessingTimeoutNotLessThanLeaseTtl { .. })
    ));
  }
}

#[tokio::test(flavor = "current_thread")]
async fn passes_completed_record_ttl_to_the_store() {
  let mut dispatcher = EventDispatcher::new();
  dispatcher
    .register(|_event: TestEvent| async { Ok::<(), HandlerError>(()) })
    .unwrap();

  let dedup_store = Arc::new(RecordingDedupStore::default());
  let completed_record_ttl = Duration::from_secs(86_400);
  let config = IncomingEventProcessorConfig::try_new(
    "realtime-node-1",
    Duration::from_secs(20),
    Duration::from_secs(30),
    completed_record_ttl,
  )
  .unwrap();
  let processor = IncomingEventProcessor::new(Arc::new(dispatcher), dedup_store.clone(), config);
  let message = EventMessage::try_from_event(&TestEvent).unwrap();

  let outcome = processor.process(&message).await.unwrap();

  assert_eq!(outcome, IncomingEventOutcome::Applied);
  assert_eq!(
    *dedup_store.completed_record_ttl.lock().unwrap(),
    Some(completed_record_ttl)
  );
  assert_eq!(*dedup_store.released_token.lock().unwrap(), None);
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn processing_timeout_releases_lease_and_does_not_complete() {
  let mut dispatcher = EventDispatcher::new();
  let handler_dropped = Arc::new(AtomicBool::new(false));
  dispatcher
    .register({
      let handler_dropped = Arc::clone(&handler_dropped);

      move |_event: TestEvent| {
        let drop_signal = HandlerDropSignal(Arc::clone(&handler_dropped));

        async move {
          let _drop_signal = drop_signal;

          std::future::pending::<Result<(), HandlerError>>().await
        }
      }
    })
    .unwrap();

  let dedup_store = Arc::new(RecordingDedupStore::default());
  let processing_timeout = Duration::from_secs(20);
  let config = IncomingEventProcessorConfig::try_new(
    "realtime-node-1",
    processing_timeout,
    Duration::from_secs(30),
    Duration::from_secs(86_400),
  )
  .unwrap();
  let processor = IncomingEventProcessor::new(Arc::new(dispatcher), dedup_store.clone(), config);
  let message = EventMessage::try_from_event(&TestEvent).unwrap();

  let error = processor.process(&message).await.unwrap_err();

  assert_eq!(error.class(), ProcessingErrorClass::Retryable);
  assert!(matches!(
    error,
    IncomingEventError::ProcessingTimeout {
      timeout,
      release_error: None,
    } if timeout == processing_timeout
  ));
  assert!(dedup_store.released_token.lock().unwrap().is_some());
  assert_eq!(*dedup_store.completed_record_ttl.lock().unwrap(), None);
  assert!(handler_dropped.load(Ordering::SeqCst));
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn processing_timeout_preserves_release_error() {
  let mut dispatcher = EventDispatcher::new();
  dispatcher
    .register(|_event: TestEvent| async {
      std::future::pending::<Result<(), HandlerError>>().await
    })
    .unwrap();

  let dedup_store = Arc::new(RecordingDedupStore {
    release_fails: true,
    ..Default::default()
  });
  let processing_timeout = Duration::from_secs(20);
  let processor = IncomingEventProcessor::new(
    Arc::new(dispatcher),
    dedup_store,
    IncomingEventProcessorConfig::try_new(
      "realtime-node-1",
      processing_timeout,
      Duration::from_secs(30),
      Duration::from_secs(86_400),
    )
    .unwrap(),
  );
  let message = EventMessage::try_from_event(&TestEvent).unwrap();

  let error = processor.process(&message).await.unwrap_err();

  assert_eq!(error.class(), ProcessingErrorClass::Retryable);
  assert!(matches!(
    error.release_error(),
    Some(DedupStoreError::LeaseLost)
  ));
  assert!(matches!(
    error,
    IncomingEventError::ProcessingTimeout { timeout, .. }
      if timeout == processing_timeout
  ));
}
