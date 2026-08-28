use event_bus::{
  DeliveryClass, EVENT_BUS_NAMESPACE_VERSION, EVENT_BUS_SUBSYSTEM, EventBusError, EventMessage,
};
use support::app::AppNamespace;

use crate::config::JetStreamPublisherConfig;
use crate::error::EventSubjectError;
use crate::publisher::prepare_message;
use crate::subject::event_subject;

const EVENT_BYTES: &[u8] = br#"{
    "eventId": "550e8400-e29b-41d4-a716-446655440000",
    "eventName": "realtime.channel_message_submitted",
    "schemaVersion": 1,
    "payload": {
        "channel": "chat.42",
        "messages": [{"text": "hello"}]
    }
}"#;

fn event_message() -> EventMessage {
  EventMessage::from_bytes(EVENT_BYTES).expect("test event envelope must be valid")
}

fn app_namespace() -> AppNamespace {
  AppNamespace::try_new(
    "mxt_realtime",
    "production",
    EVENT_BUS_SUBSYSTEM,
    EVENT_BUS_NAMESPACE_VERSION,
  )
  .expect("application namespace must be valid")
}

#[test]
fn builds_subject_prefix_from_app_namespace() {
  let namespace = app_namespace();
  let config = JetStreamPublisherConfig::new(&namespace);

  assert_eq!(config.subject_prefix(), namespace.as_str());
}

#[test]
fn maps_all_nodes_events_to_fanout_subjects() {
  let subject = event_subject(
    "mxt_realtime.production.event-bus.v1",
    "realtime.channel_message_submitted",
    DeliveryClass::AllNodes,
  )
  .expect("AllNodes subject must be supported");

  assert_eq!(
    subject,
    "mxt_realtime.production.event-bus.v1.all.realtime.channel_message_submitted"
  );
}

#[test]
fn maps_work_queue_events_to_work_subjects() {
  let subject = event_subject(
    "mxt_realtime.production.event-bus.v1",
    "jobs.audit_requested",
    DeliveryClass::WorkQueue,
  )
  .expect("WorkQueue subject must be supported");

  assert_eq!(
    subject,
    "mxt_realtime.production.event-bus.v1.work.jobs.audit_requested"
  );
}

#[test]
fn rejects_local_only_delivery_for_jetstream() {
  let error = event_subject(
    "mxt_realtime.production.event-bus.v1",
    "realtime.websocket_connected",
    DeliveryClass::LocalOnly,
  )
  .expect_err("LocalOnly event must not have a JetStream subject");

  assert!(matches!(error, EventSubjectError::UnsupportedDeliveryClass));
}

#[test]
fn rejects_invalid_event_names() {
  for event_name in [
    "",
    ".realtime.event",
    "realtime.event.",
    "realtime..event",
    "realtime event",
    "realtime.*",
    "realtime.>",
  ] {
    let error = event_subject(
      "mxt_realtime.production.event-bus.v1",
      event_name,
      DeliveryClass::AllNodes,
    )
    .expect_err("invalid event name must be rejected");

    assert!(matches!(error, EventSubjectError::InvalidEventName { .. }));
  }
}

#[test]
fn prepares_subject_and_preserves_event_envelope() {
  let config = JetStreamPublisherConfig::new(&app_namespace());
  let event = event_message();

  let outgoing = prepare_message(&config, &event, DeliveryClass::AllNodes)
    .expect("event publication must be prepared");

  assert_eq!(
    outgoing.subject(),
    "mxt_realtime.production.event-bus.v1.all.realtime.channel_message_submitted"
  );

  let decoded = EventMessage::from_bytes(outgoing.payload())
    .expect("prepared payload must contain an EventMessage");

  assert_eq!(decoded, event);
}

#[test]
fn reports_local_only_preparation_as_publisher_error() {
  let config = JetStreamPublisherConfig::new(&app_namespace());
  let event = event_message();

  let error = prepare_message(&config, &event, DeliveryClass::LocalOnly)
    .expect_err("LocalOnly event must not be prepared for JetStream");

  assert!(matches!(error, EventBusError::Publisher(_)));
}
