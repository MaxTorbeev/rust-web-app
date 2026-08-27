use std::sync::{Arc, Mutex};
use std::time::Duration;

use event_bus::{
  DedupClaim, DedupKey, DedupLease, DedupStore, DedupStoreError, DedupStoreFuture, Event,
  EventDispatcher, EventMessage, HandlerError, IncomingEventOutcome, IncomingEventProcessor,
  IncomingEventProcessorConfigError,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
struct TestEvent;

impl Event for TestEvent {
  const NAME: &'static str = "test.incoming_event";
}

struct UnusedDedupStore;

impl DedupStore for UnusedDedupStore {
  fn claim<'a>(
    &'a self,
    _key: &'a DedupKey,
    _lease_ttl: Duration,
  ) -> DedupStoreFuture<'a, DedupClaim> {
    Box::pin(async { Err(DedupStoreError::LeaseLost) })
  }

  fn complete<'a>(
    &'a self,
    _lease: &'a DedupLease,
    _completed_ttl: Duration,
  ) -> DedupStoreFuture<'a, ()> {
    Box::pin(async { Err(DedupStoreError::LeaseLost) })
  }

  fn release<'a>(&'a self, _lease: &'a DedupLease) -> DedupStoreFuture<'a, ()> {
    Box::pin(async { Err(DedupStoreError::LeaseLost) })
  }
}

#[derive(Default)]
struct RecordingDedupStore {
  completed_record_ttl: Mutex<Option<Duration>>,
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

  fn release<'a>(&'a self, _lease: &'a DedupLease) -> DedupStoreFuture<'a, ()> {
    Box::pin(async { Ok(()) })
  }
}

fn dependencies() -> (Arc<EventDispatcher>, Arc<dyn DedupStore>) {
  let dispatcher = Arc::new(EventDispatcher::new());
  let dedup_store: Arc<dyn DedupStore> = Arc::new(UnusedDedupStore);

  (dispatcher, dedup_store)
}

#[test]
fn accepts_valid_configuration() {
  let (dispatcher, dedup_store) = dependencies();

  let processor = IncomingEventProcessor::try_new(
    dispatcher,
    dedup_store,
    "realtime-node-1",
    Duration::from_secs(30),
    Duration::from_secs(86_400),
  );

  assert!(processor.is_ok());
}

#[test]
fn rejects_empty_scope() {
  let (dispatcher, dedup_store) = dependencies();

  let result = IncomingEventProcessor::try_new(
    dispatcher,
    dedup_store,
    "",
    Duration::from_secs(30),
    Duration::from_secs(86_400),
  );

  assert!(matches!(
    result,
    Err(IncomingEventProcessorConfigError::EmptyScope)
  ));
}

#[test]
fn rejects_zero_lease_ttl() {
  let (dispatcher, dedup_store) = dependencies();

  let result = IncomingEventProcessor::try_new(
    dispatcher,
    dedup_store,
    "realtime-node-1",
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
  let (dispatcher, dedup_store) = dependencies();

  let result = IncomingEventProcessor::try_new(
    dispatcher,
    dedup_store,
    "realtime-node-1",
    Duration::from_secs(30),
    Duration::ZERO,
  );

  assert!(matches!(
    result,
    Err(IncomingEventProcessorConfigError::ZeroCompletedRecordTtl)
  ));
}

#[tokio::test(flavor = "current_thread")]
async fn passes_completed_record_ttl_to_the_store() {
  let mut dispatcher = EventDispatcher::new();
  dispatcher
    .register(|_event: TestEvent| async { Ok::<(), HandlerError>(()) })
    .unwrap();

  let dedup_store = Arc::new(RecordingDedupStore::default());
  let completed_record_ttl = Duration::from_secs(86_400);
  let processor = IncomingEventProcessor::try_new(
    Arc::new(dispatcher),
    dedup_store.clone(),
    "realtime-node-1",
    Duration::from_secs(30),
    completed_record_ttl,
  )
  .unwrap();
  let message = EventMessage::try_from_event(&TestEvent).unwrap();

  let outcome = processor.process(&message).await.unwrap();

  assert_eq!(outcome, IncomingEventOutcome::Applied);
  assert_eq!(
    *dedup_store.completed_record_ttl.lock().unwrap(),
    Some(completed_record_ttl)
  );
}
