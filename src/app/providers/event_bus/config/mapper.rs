use std::time::Duration;

use event_bus::{EVENT_BUS_NAMESPACE_VERSION, EVENT_BUS_SUBSYSTEM, IncomingEventProcessorConfig};
use event_bus_dedup_redis::RedisDedupStoreConfig;
use event_bus_jetstream::{JetStreamIncomingConsumerConfig, JetStreamSubjectConfig};
use nats_client::{ConsumerConfig, NatsConfig, StreamConfig, StreamLimits};
use support::app::AppNamespace;

use super::{
  config::{EventBusConfig, JetStreamEventBusConfig},
  mapper_error::EventBusConfigMapperError,
  source::{
    EventBusConfigSource, EventBusDriverSource, EventProcessingConfigSource, NatsConfigSource,
    NatsConsumerConfigSource, NatsStreamConfigSource,
  },
};

pub(in crate::app::providers::event_bus) fn map(
  source: EventBusConfigSource,
) -> Result<EventBusConfig, EventBusConfigMapperError> {
  match source.driver {
    EventBusDriverSource::Local => Ok(EventBusConfig::Local),
    EventBusDriverSource::Nats => map_nats(source).map(EventBusConfig::JetStream),
  }
}

fn map_nats(
  source: EventBusConfigSource,
) -> Result<JetStreamEventBusConfig, EventBusConfigMapperError> {
  let EventBusConfigSource {
    app,
    app_environment,
    nats,
    ..
  } = source;

  let NatsConfigSource {
    servers,
    node_id,
    stream,
    consumer,
    processing,
  } = nats;

  let NatsStreamConfigSource {
    name: stream_name,
    max_messages,
    max_message_size_bytes,
    max_bytes,
    max_age_seconds,
    replicas,
    duplicate_window_seconds,
  } = stream;

  let NatsConsumerConfigSource {
    ack_wait_seconds,
    max_deliver,
    retry_delay_seconds,
  } = consumer;

  let EventProcessingConfigSource {
    timeout_seconds,
    lease_ttl_seconds,
    completed_record_ttl_seconds,
  } = processing;

  let app = require(app, "APP")?;
  let app_environment = require(app_environment, "APP_ENV")?;
  let node_id = require(node_id, "REALTIME_NODE_ID")?;
  let stream_name = require(stream_name, "NATS_STREAM_NAME")?;

  let max_age = Duration::from_secs(max_age_seconds);
  let duplicate_window = Duration::from_secs(duplicate_window_seconds);
  let ack_wait = Duration::from_secs(ack_wait_seconds);
  let retry_delay = Duration::from_secs(retry_delay_seconds);
  let processing_timeout = Duration::from_secs(timeout_seconds);
  let lease_ttl = Duration::from_secs(lease_ttl_seconds);
  let completed_record_ttl = Duration::from_secs(completed_record_ttl_seconds);

  // Todo move to app service provider
  let namespace = AppNamespace::try_new(
    app,
    app_environment,
    EVENT_BUS_SUBSYSTEM,
    EVENT_BUS_NAMESPACE_VERSION,
  )?;

  let nats = NatsConfig::try_new(servers.unwrap_or_default())?;
  let subjects = JetStreamSubjectConfig::new(&namespace);
  let dedup = RedisDedupStoreConfig::new(&namespace);
  let all_nodes_subject = subjects.all_nodes_subject_filter();

  let limits = StreamLimits::try_new(max_messages, max_message_size_bytes, max_bytes, max_age)?;

  let stream = StreamConfig::try_new(
    stream_name.clone(),
    [all_nodes_subject.clone()],
    limits,
    replicas,
    duplicate_window,
  )?;

  let consumer = ConsumerConfig::try_new(
    stream_name,
    node_id.clone(),
    all_nodes_subject,
    ack_wait,
    max_deliver,
  )?;

  let processor = IncomingEventProcessorConfig::try_new(
    node_id,
    processing_timeout,
    lease_ttl,
    completed_record_ttl,
  )?;

  let incoming = JetStreamIncomingConsumerConfig::try_new(retry_delay)?;

  validate_relationships(ack_wait, lease_ttl, max_age, completed_record_ttl)?;

  Ok(JetStreamEventBusConfig {
    nats,
    stream,
    consumer,
    subjects,
    dedup,
    processor,
    incoming,
  })
}

fn validate_relationships(
  ack_wait: Duration,
  lease_ttl: Duration,
  max_age: Duration,
  completed_record_ttl: Duration,
) -> Result<(), EventBusConfigMapperError> {
  if ack_wait <= lease_ttl {
    return Err(EventBusConfigMapperError::invalid(format!(
      "JetStream consumer ack_wait ({ack_wait:?}) must be greater than deduplication lease_ttl ({lease_ttl:?})"
    )));
  }

  if completed_record_ttl < max_age {
    return Err(EventBusConfigMapperError::invalid(format!(
      "deduplication completed_record_ttl ({completed_record_ttl:?}) must not be shorter than stream max_age ({max_age:?})"
    )));
  }

  Ok(())
}

fn require<T>(value: Option<T>, variable: &'static str) -> Result<T, EventBusConfigMapperError> {
  value.ok_or_else(|| {
    EventBusConfigMapperError::invalid(format!("{variable} is required when EVENT_BUS_DRIVER=nats"))
  })
}
