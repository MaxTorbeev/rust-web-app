use std::time::Duration;

use async_nats::jetstream::{
  consumer::{AckPolicy, DeliverPolicy, IntoConsumerConfig},
  stream::{RetentionPolicy, StorageType},
};

use crate::{
  ConsumerConfig, ConsumerConfigError, NatsConfig, NatsConfigError, StreamConfig,
  StreamConfigError, StreamLimits, StreamLimitsError,
};

fn stream_limits() -> StreamLimits {
  StreamLimits::try_new(
    100_000,
    1_048_576,
    1_073_741_824,
    Duration::from_secs(86_400),
  )
  .expect("stream limits must be valid")
}

#[test]
fn validates_nats_server_list() {
  let config = NatsConfig::try_new(["nats://nats-1:4222", "nats://nats-2:4222"])
    .expect("server list must be valid");

  assert_eq!(
    config.servers(),
    &["nats://nats-1:4222", "nats://nats-2:4222"]
  );
  assert_eq!(
    NatsConfig::try_new(Vec::<String>::new()).unwrap_err(),
    NatsConfigError::NoServers
  );
  assert!(matches!(
    NatsConfig::try_new(["nats://nats 1:4222"]).unwrap_err(),
    NatsConfigError::InvalidServer { index: 0, .. }
  ));
}

#[test]
fn converts_valid_consumer_config_to_explicit_pull_policy() {
  let config = ConsumerConfig::try_new(
    "EVENTS",
    "realtime-node-1",
    "mxt.production.events.all.>",
    Duration::from_secs(30),
    5,
    512,
  )
  .expect("consumer config must be valid");

  let driver = config.to_driver_config();

  assert_eq!(driver.durable_name.as_deref(), Some("realtime-node-1"));
  assert_eq!(driver.deliver_policy, DeliverPolicy::New);
  assert_eq!(driver.ack_policy, AckPolicy::Explicit);
  assert_eq!(driver.ack_wait, Duration::from_secs(30));
  assert_eq!(driver.max_deliver, 5);
  assert_eq!(driver.max_ack_pending, 512);
  assert_eq!(driver.filter_subject, "mxt.production.events.all.>");
}

#[test]
fn rejects_invalid_consumer_config() {
  let valid = || {
    ConsumerConfig::try_new(
      "EVENTS",
      "realtime-node-1",
      "mxt.production.events.all.>",
      Duration::from_secs(30),
      5,
      512,
    )
  };

  assert!(valid().is_ok());
  assert!(matches!(
    ConsumerConfig::try_new(
      "EVENTS.invalid",
      "realtime-node-1",
      "events.>",
      Duration::from_secs(30),
      5,
      512,
    )
    .unwrap_err(),
    ConsumerConfigError::InvalidStreamName { .. }
  ));
  assert!(matches!(
    ConsumerConfig::try_new(
      "EVENTS",
      "realtime node",
      "events.>",
      Duration::from_secs(30),
      5,
      512,
    )
    .unwrap_err(),
    ConsumerConfigError::InvalidDurableName { .. }
  ));
  assert!(matches!(
    ConsumerConfig::try_new(
      "EVENTS",
      "realtime-node-1",
      "events.>.invalid",
      Duration::from_secs(30),
      5,
      512,
    )
    .unwrap_err(),
    ConsumerConfigError::InvalidFilterSubject { .. }
  ));
  assert_eq!(
    ConsumerConfig::try_new(
      "EVENTS",
      "realtime-node-1",
      "events.>",
      Duration::ZERO,
      5,
      512,
    )
    .unwrap_err(),
    ConsumerConfigError::ZeroAckWait
  );
}

#[test]
fn detects_incompatible_consumer_configuration() {
  let config = ConsumerConfig::try_new(
    "EVENTS",
    "realtime-node-1",
    "events.all.>",
    Duration::from_secs(30),
    5,
    512,
  )
  .expect("consumer config must be valid");
  let mut driver = config.to_driver_config();
  driver.max_deliver = 10;
  driver.filter_subject = "events.work.>".to_owned();
  let driver = driver.into_consumer_config();

  assert_eq!(
    config.incompatible_fields(&driver),
    ["max_deliver", "filter_subject"]
  );
}

#[test]
fn validates_stream_limits() {
  assert!(StreamLimits::try_new(1, 1024, 1024, Duration::from_secs(1)).is_ok());
  assert_eq!(
    StreamLimits::try_new(0, 1024, 1024, Duration::from_secs(1)).unwrap_err(),
    StreamLimitsError::InvalidMaxMessages { max_messages: 0 }
  );
  assert_eq!(
    StreamLimits::try_new(1, 1024, 1024, Duration::ZERO).unwrap_err(),
    StreamLimitsError::ZeroMaxAge
  );
}

#[test]
fn converts_valid_stream_config_to_bounded_file_storage() {
  let config = StreamConfig::try_new(
    "EVENTS",
    [
      "mxt.production.events.all.>",
      "mxt.production.events.work.>",
    ],
    stream_limits(),
    3,
    Duration::from_secs(120),
  )
  .expect("stream config must be valid");

  let driver = config.to_driver_config();

  assert_eq!(driver.name, "EVENTS");
  assert_eq!(driver.retention, RetentionPolicy::Limits);
  assert_eq!(driver.storage, StorageType::File);
  assert_eq!(driver.num_replicas, 3);
  assert_eq!(driver.duplicate_window, Duration::from_secs(120));
  assert_eq!(driver.max_messages, 100_000);
  assert_eq!(driver.max_message_size, 1_048_576);
  assert_eq!(driver.max_bytes, 1_073_741_824);
  assert_eq!(driver.max_age, Duration::from_secs(86_400));
}

#[test]
fn rejects_invalid_stream_config() {
  assert!(matches!(
    StreamConfig::try_new(
      "EVENTS.invalid",
      ["events.>"],
      stream_limits(),
      1,
      Duration::from_secs(120),
    )
    .unwrap_err(),
    StreamConfigError::InvalidName { .. }
  ));
  assert_eq!(
    StreamConfig::try_new(
      "EVENTS",
      ["events.>", "events.>"],
      stream_limits(),
      1,
      Duration::from_secs(120),
    )
    .unwrap_err(),
    StreamConfigError::DuplicateSubjects
  );
  assert_eq!(
    StreamConfig::try_new(
      "EVENTS",
      ["events.>"],
      stream_limits(),
      0,
      Duration::from_secs(120),
    )
    .unwrap_err(),
    StreamConfigError::InvalidReplicas { replicas: 0 }
  );
  assert_eq!(
    StreamConfig::try_new("EVENTS", ["events.>"], stream_limits(), 1, Duration::ZERO,).unwrap_err(),
    StreamConfigError::ZeroDuplicateWindow
  );
}

#[test]
fn compares_stream_subjects_without_ordering_them() {
  let config = StreamConfig::try_new(
    "EVENTS",
    ["events.all.>", "events.work.>"],
    stream_limits(),
    1,
    Duration::from_secs(120),
  )
  .expect("stream config must be valid");
  let mut driver = config.to_driver_config();
  driver.subjects.reverse();

  assert!(config.incompatible_fields(&driver).is_empty());

  driver.max_age = Duration::from_secs(60);
  assert_eq!(config.incompatible_fields(&driver), ["max_age"]);
}
