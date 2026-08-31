use std::time::Duration;

use confique::Config;
use event_bus::{EVENT_BUS_NAMESPACE_VERSION, EVENT_BUS_SUBSYSTEM};
use event_bus_dedup_redis::RedisDedupStoreConfig;
use event_bus_jetstream::JetStreamSubjectConfig;
use support::app::AppNamespace;

use super::{EventBusConfig, EventBusConfigSource, EventBusDriverSource, map};

#[test]
fn maps_local_driver() {
  let source = load_source();

  assert!(matches!(map(source), Ok(EventBusConfig::Local)));
}

#[test]
fn maps_nats_driver_to_complete_jetstream_config() {
  let mut source = load_source();
  source.driver = EventBusDriverSource::Nats;
  source.app = Some("mxt_realtime".to_owned());
  source.app_environment = Some("local".to_owned());

  let EventBusConfig::JetStream(config) = map(source).expect("NATS config must be valid") else {
    panic!("NATS driver must map to JetStream config");
  };

  let namespace = AppNamespace::try_new(
    "mxt_realtime",
    "local",
    EVENT_BUS_SUBSYSTEM,
    EVENT_BUS_NAMESPACE_VERSION,
  )
  .expect("test namespace must be valid");

  assert_eq!(config.nats.servers(), ["nats://127.0.0.1:4222"]);
  assert_eq!(config.subjects, JetStreamSubjectConfig::new(&namespace));
  assert_eq!(config.dedup, RedisDedupStoreConfig::new(&namespace));
  assert_eq!(config.processor.scope(), "realtime-local-1");
  assert_eq!(
    config.processor.processing_timeout(),
    Duration::from_secs(20)
  );
  assert_eq!(config.processor.lease_ttl(), Duration::from_secs(30));
  assert_eq!(
    config.processor.completed_record_ttl(),
    Duration::from_secs(86_400)
  );
  assert_eq!(config.incoming.retry_delay(), Duration::from_secs(5));

  // These fields are intentionally consumed later by the provider. Reaching
  // this point proves that both typed constructors accepted the mapped values.
  let _ = config.stream;
  let _ = config.consumer;
}

#[test]
fn maps_nats_driver_without_toml_file() {
  let mut source = EventBusConfigSource::builder()
    .load()
    .expect("built-in Event Bus defaults must load");

  source.driver = EventBusDriverSource::Nats;
  source.app = Some("mxt_realtime".to_owned());
  source.app_environment = Some("staging".to_owned());
  source.nats.servers = Some(vec!["nats://nats:4222".to_owned()]);
  source.nats.node_id = Some("realtime-staging-1".to_owned());
  source.nats.stream.name = Some("MXT_REALTIME_EVENTS".to_owned());
  source.nats.stream.replicas = 3;

  let EventBusConfig::JetStream(config) = map(source).expect("env-only NATS config must be valid")
  else {
    panic!("NATS driver must map to JetStream config");
  };

  assert_eq!(config.nats.servers(), ["nats://nats:4222"]);
  assert_eq!(config.processor.scope(), "realtime-staging-1");
  assert_eq!(
    config.processor.processing_timeout(),
    Duration::from_secs(20)
  );
  assert_eq!(config.processor.lease_ttl(), Duration::from_secs(30));
  assert_eq!(
    config.processor.completed_record_ttl(),
    Duration::from_secs(86_400)
  );
  assert_eq!(config.incoming.retry_delay(), Duration::from_secs(5));

  let _ = config.stream;
  let _ = config.consumer;
}

fn load_source() -> EventBusConfigSource {
  EventBusConfigSource::builder()
    .file("config/event_bus.toml")
    .load()
    .expect("example event bus config must load")
}
