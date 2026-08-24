use event_bus::{DeliveryClass, EventBusError, EventMessage};

use crate::config::JetStreamPublisherConfig;
use crate::error::{EventSubjectError, JetStreamPublisherConfigError};
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

#[test]
fn builds_subject_prefix_from_explicit_namespace() {
    let config = JetStreamPublisherConfig::try_new("mxt_realtime", "production")
        .expect("publisher config must be valid");

    assert_eq!(config.subject_prefix(), "mxt_realtime.production.events");
}

#[test]
fn rejects_invalid_namespace_components() {
    for app_name in ["", "mxt realtime", "mxt.realtime", "mxt*", "mxt>", "мхт"] {
        let error = JetStreamPublisherConfig::try_new(app_name, "production")
            .expect_err("invalid APP value must be rejected");

        assert!(matches!(
            error,
            JetStreamPublisherConfigError::InvalidNamespaceComponent {
                component: "APP",
                ..
            }
        ));
    }

    for app_environment in ["", "production eu", "production.eu", "prod*", "prod>"] {
        let error = JetStreamPublisherConfig::try_new("mxt_realtime", app_environment)
            .expect_err("invalid APP_ENV value must be rejected");

        assert!(matches!(
            error,
            JetStreamPublisherConfigError::InvalidNamespaceComponent {
                component: "APP_ENV",
                ..
            }
        ));
    }
}

#[test]
fn maps_all_nodes_events_to_fanout_subjects() {
    let subject = event_subject(
        "mxt_realtime.production.events",
        "realtime.channel_message_submitted",
        DeliveryClass::AllNodes,
    )
    .expect("AllNodes subject must be supported");

    assert_eq!(
        subject,
        "mxt_realtime.production.events.all.realtime.channel_message_submitted"
    );
}

#[test]
fn maps_work_queue_events_to_work_subjects() {
    let subject = event_subject(
        "mxt_realtime.production.events",
        "jobs.audit_requested",
        DeliveryClass::WorkQueue,
    )
    .expect("WorkQueue subject must be supported");

    assert_eq!(
        subject,
        "mxt_realtime.production.events.work.jobs.audit_requested"
    );
}

#[test]
fn rejects_local_only_delivery_for_jetstream() {
    let error = event_subject(
        "mxt_realtime.production.events",
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
            "mxt_realtime.production.events",
            event_name,
            DeliveryClass::AllNodes,
        )
        .expect_err("invalid event name must be rejected");

        assert!(matches!(error, EventSubjectError::InvalidEventName { .. }));
    }
}

#[test]
fn prepares_subject_and_preserves_event_envelope() {
    let config = JetStreamPublisherConfig::try_new("mxt_realtime", "production")
        .expect("publisher config must be valid");
    let event = event_message();

    let outgoing = prepare_message(&config, &event, DeliveryClass::AllNodes)
        .expect("event publication must be prepared");

    assert_eq!(
        outgoing.subject(),
        "mxt_realtime.production.events.all.realtime.channel_message_submitted"
    );

    let decoded = EventMessage::from_bytes(outgoing.payload())
        .expect("prepared payload must contain an EventMessage");

    assert_eq!(decoded, event);
}

#[test]
fn reports_local_only_preparation_as_publisher_error() {
    let config = JetStreamPublisherConfig::try_new("mxt_realtime", "production")
        .expect("publisher config must be valid");
    let event = event_message();

    let error = prepare_message(&config, &event, DeliveryClass::LocalOnly)
        .expect_err("LocalOnly event must not be prepared for JetStream");

    assert!(matches!(error, EventBusError::Publisher(_)));
}
