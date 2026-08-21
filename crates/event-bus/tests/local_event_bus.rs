use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use event_bus::{
    DeliveryClass, Event, EventBus, EventBusError, EventDispatcher, EventMessage,
    EventPublishFuture, EventPublisher,
};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::json;
use thiserror::Error;
use tokio::sync::oneshot;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct DistributedEvent {
    value: String,
}

impl Event for DistributedEvent {
    const NAME: &'static str = "test.distributed";
    const VERSION: u16 = 3;
    const DELIVERY: DeliveryClass = DeliveryClass::AllNodes;
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct LocalEvent {
    value: String,
}

impl Event for LocalEvent {
    const NAME: &'static str = "test.local";
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct WorkQueueEvent;

impl Event for WorkQueueEvent {
    const NAME: &'static str = "test.work_queue";
    const DELIVERY: DeliveryClass = DeliveryClass::WorkQueue;
}

#[derive(Debug, Deserialize)]
struct FailingEncodeEvent;

impl Serialize for FailingEncodeEvent {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Err(serde::ser::Error::custom("test encode failure"))
    }
}

impl Event for FailingEncodeEvent {
    const NAME: &'static str = "test.failing_encode";
    const DELIVERY: DeliveryClass = DeliveryClass::AllNodes;
}

#[derive(Default)]
struct RecordingPublisher {
    publications: Mutex<Vec<(EventMessage, DeliveryClass)>>,
}

impl RecordingPublisher {
    fn publications(&self) -> Vec<(EventMessage, DeliveryClass)> {
        self.publications
            .lock()
            .expect("publication lock must not be poisoned")
            .clone()
    }
}

impl EventPublisher for RecordingPublisher {
    fn publish<'a>(
        &'a self,
        message: &'a EventMessage,
        delivery: DeliveryClass,
    ) -> EventPublishFuture<'a> {
        Box::pin(async move {
            self.publications
                .lock()
                .expect("publication lock must not be poisoned")
                .push((message.clone(), delivery));

            Ok(())
        })
    }
}

#[derive(Debug, Error)]
#[error("test publisher failed")]
struct PublisherFailed;

#[derive(Default)]
struct FailingPublisher {
    calls: AtomicUsize,
}

impl EventPublisher for FailingPublisher {
    fn publish<'a>(
        &'a self,
        _message: &'a EventMessage,
        _delivery: DeliveryClass,
    ) -> EventPublishFuture<'a> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);

            Err(EventBusError::publisher(PublisherFailed))
        })
    }
}

#[tokio::test(flavor = "current_thread")]
async fn publish_builds_one_envelope_and_returns_its_id() {
    let distributed = Arc::new(RecordingPublisher::default());
    let event_bus =
        EventBus::with_distributed_publisher(Arc::new(EventDispatcher::new()), distributed.clone());

    let receipt = event_bus
        .publish(DistributedEvent {
            value: "payload".to_owned(),
        })
        .await
        .expect("event must publish");

    let publications = distributed.publications();
    assert_eq!(publications.len(), 1);

    let (message, delivery) = &publications[0];
    assert_eq!(*delivery, DeliveryClass::AllNodes);
    assert_eq!(message.event_id(), receipt.event_id);
    assert_eq!(message.event_name(), DistributedEvent::NAME);
    assert_eq!(message.schema_version(), DistributedEvent::VERSION);
    assert_eq!(message.payload(), &json!({ "value": "payload" }));
}

#[tokio::test(flavor = "current_thread")]
async fn local_only_event_uses_local_publisher() {
    let received = Arc::new(Mutex::new(Vec::new()));
    let handler_received = Arc::clone(&received);
    let mut dispatcher = EventDispatcher::new();

    dispatcher
        .register(move |event: LocalEvent| {
            let handler_received = Arc::clone(&handler_received);

            async move {
                handler_received
                    .lock()
                    .expect("received event lock must not be poisoned")
                    .push(event);

                Ok(())
            }
        })
        .expect("handler must register");

    let distributed = Arc::new(RecordingPublisher::default());
    let event_bus = EventBus::with_distributed_publisher(Arc::new(dispatcher), distributed.clone());

    event_bus
        .publish(LocalEvent {
            value: "local".to_owned(),
        })
        .await
        .expect("local event must publish");

    assert_eq!(
        *received
            .lock()
            .expect("received event lock must not be poisoned"),
        vec![LocalEvent {
            value: "local".to_owned(),
        }],
    );
    assert!(distributed.publications().is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn publish_does_not_retry_publisher_error() {
    let distributed = Arc::new(FailingPublisher::default());
    let event_bus =
        EventBus::with_distributed_publisher(Arc::new(EventDispatcher::new()), distributed.clone());

    let result = event_bus
        .publish(DistributedEvent {
            value: "payload".to_owned(),
        })
        .await;

    assert!(matches!(result, Err(EventBusError::Publisher(_))));
    assert_eq!(distributed.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn encoding_failure_does_not_call_a_publisher() {
    let distributed = Arc::new(RecordingPublisher::default());
    let event_bus =
        EventBus::with_distributed_publisher(Arc::new(EventDispatcher::new()), distributed.clone());

    let result = event_bus.publish(FailingEncodeEvent).await;

    assert!(matches!(result, Err(EventBusError::Encode(_))));
    assert!(distributed.publications().is_empty());
}

#[test]
fn event_message_wire_round_trip_preserves_the_envelope() {
    let event_id =
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("event id must be valid");
    let message = EventMessage::try_from_event_with_id(
        event_id,
        &DistributedEvent {
            value: "payload".to_owned(),
        },
    )
    .expect("event message must encode");

    let bytes = message.to_bytes().expect("event message must serialize");
    let decoded = EventMessage::from_bytes(&bytes).expect("event message must deserialize");

    assert_eq!(decoded, message);
}

#[test]
fn event_message_rejects_corrupt_transport_bytes() {
    let result = EventMessage::from_bytes(b"not-json");

    assert!(matches!(result, Err(EventBusError::Decode(_))));
}

#[tokio::test(flavor = "current_thread")]
async fn work_queue_event_uses_distributed_publisher() {
    let distributed = Arc::new(RecordingPublisher::default());
    let event_bus =
        EventBus::with_distributed_publisher(Arc::new(EventDispatcher::new()), distributed.clone());

    event_bus
        .publish(WorkQueueEvent)
        .await
        .expect("work queue event must publish");

    let publications = distributed.publications();
    assert_eq!(publications.len(), 1);
    assert_eq!(publications[0].1, DeliveryClass::WorkQueue);
}

#[tokio::test(flavor = "current_thread")]
async fn dispatcher_decodes_and_awaits_the_typed_handler() {
    let received = Arc::new(Mutex::new(Vec::new()));
    let handler_received = Arc::clone(&received);
    let (started_sender, started_receiver) = oneshot::channel();
    let started_sender = Arc::new(Mutex::new(Some(started_sender)));
    let handler_started_sender = Arc::clone(&started_sender);
    let (release_sender, release_receiver) = oneshot::channel();
    let release_receiver = Arc::new(Mutex::new(Some(release_receiver)));
    let handler_release_receiver = Arc::clone(&release_receiver);
    let mut dispatcher = EventDispatcher::new();

    dispatcher
        .register(move |event: DistributedEvent| {
            let handler_received = Arc::clone(&handler_received);
            let started_sender = handler_started_sender
                .lock()
                .expect("started sender lock must not be poisoned")
                .take()
                .expect("handler must run once");
            let release_receiver = handler_release_receiver
                .lock()
                .expect("release receiver lock must not be poisoned")
                .take()
                .expect("handler must run once");

            async move {
                started_sender
                    .send(())
                    .expect("test must wait for the handler to start");
                release_receiver
                    .await
                    .expect("test must release the handler");

                handler_received
                    .lock()
                    .expect("received event lock must not be poisoned")
                    .push(event);

                Ok(())
            }
        })
        .expect("handler must register");

    let message = EventMessage::try_from_event_with_id(
        Uuid::new_v4(),
        &DistributedEvent {
            value: "decoded".to_owned(),
        },
    )
    .expect("event message must encode");

    let dispatcher = Arc::new(dispatcher);
    let dispatch_task = tokio::spawn({
        let dispatcher = Arc::clone(&dispatcher);

        async move { dispatcher.dispatch(&message).await }
    });

    started_receiver
        .await
        .expect("handler must report that it started");
    assert!(!dispatch_task.is_finished());

    release_sender
        .send(())
        .expect("handler must still be waiting");
    dispatch_task
        .await
        .expect("dispatch task must not panic")
        .expect("event must dispatch");

    assert_eq!(
        *received
            .lock()
            .expect("received event lock must not be poisoned"),
        vec![DistributedEvent {
            value: "decoded".to_owned(),
        }],
    );
}

#[tokio::test(flavor = "current_thread")]
async fn dispatcher_rejects_duplicate_wire_event_name() {
    #[derive(Debug, Deserialize, Serialize)]
    struct DifferentRustType;

    impl Event for DifferentRustType {
        const NAME: &'static str = DistributedEvent::NAME;
    }

    let mut dispatcher = EventDispatcher::new();
    dispatcher
        .register(|_event: DistributedEvent| async { Ok(()) })
        .expect("first handler must register");

    let result = dispatcher.register(|_event: DifferentRustType| async { Ok(()) });

    assert!(matches!(
      result,
      Err(EventBusError::HandlerAlreadyRegistered { event_name })
        if event_name == DistributedEvent::NAME
    ));
}

#[tokio::test(flavor = "current_thread")]
async fn dispatcher_rejects_unknown_version_and_invalid_payload() {
    let calls = Arc::new(AtomicUsize::new(0));
    let handler_calls = Arc::clone(&calls);
    let mut dispatcher = EventDispatcher::new();

    dispatcher
        .register(move |_event: DistributedEvent| {
            let handler_calls = Arc::clone(&handler_calls);

            async move {
                handler_calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        })
        .expect("handler must register");

    let unknown = EventMessage::new(Uuid::new_v4(), "test.unknown", 1, json!({}));
    assert!(matches!(
      dispatcher.dispatch(&unknown).await,
      Err(EventBusError::HandlerNotRegistered { event_name })
        if event_name == "test.unknown"
    ));

    let wrong_version = EventMessage::new(
        Uuid::new_v4(),
        DistributedEvent::NAME,
        DistributedEvent::VERSION + 1,
        json!({ "value": "payload" }),
    );
    assert!(matches!(
        dispatcher.dispatch(&wrong_version).await,
        Err(EventBusError::EventVersionMismatch { .. })
    ));

    let invalid_payload = EventMessage::new(
        Uuid::new_v4(),
        DistributedEvent::NAME,
        DistributedEvent::VERSION,
        json!({ "unexpected": true }),
    );
    assert!(matches!(
        dispatcher.dispatch(&invalid_payload).await,
        Err(EventBusError::Decode(_))
    ));

    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[derive(Debug, Error)]
#[error("test handler failed")]
struct HandlerFailed;

#[tokio::test(flavor = "current_thread")]
async fn local_event_bus_propagates_handler_error_without_retry() {
    let calls = Arc::new(AtomicUsize::new(0));
    let handler_calls = Arc::clone(&calls);
    let mut dispatcher = EventDispatcher::new();

    dispatcher
        .register(move |_event: DistributedEvent| {
            let handler_calls = Arc::clone(&handler_calls);

            async move {
                handler_calls.fetch_add(1, Ordering::SeqCst);

                Err(EventBusError::handler(
                    DistributedEvent::NAME,
                    HandlerFailed,
                ))
            }
        })
        .expect("handler must register");

    let event_bus = EventBus::local(Arc::new(dispatcher));
    let result = event_bus
        .publish(DistributedEvent {
            value: "payload".to_owned(),
        })
        .await;

    assert!(matches!(
      result,
      Err(EventBusError::Handler { event_name, .. })
        if event_name == DistributedEvent::NAME
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
