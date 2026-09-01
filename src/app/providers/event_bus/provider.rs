use super::{EventBusConfig, EventBusProviderError, EventBusRuntime, JetStreamEventBusConfig};
use crate::app::listeners;
use event_bus::{EventBus, EventDispatcher, HandlerRegistrationError, IncomingEventProcessor};
use event_bus_dedup_redis::RedisDedupStore;
use event_bus_jetstream::{JetStreamEventPublisher, JetStreamIncomingConsumer};
use nats_client::NatsClient;
use realtime::{Realtime, register_event_handlers};
use redis_client::RedisClient;
use std::sync::Arc;

pub struct EventBusProvider;

impl EventBusProvider {
  pub async fn build(
    redis: Arc<RedisClient>,
    realtime: Arc<Realtime>,
  ) -> Result<EventBusRuntime, EventBusProviderError> {
    let source = super::config::load()?;
    let config = super::config::map(source)?;
    let dispatcher = build_dispatcher(realtime)?;

    match config {
      EventBusConfig::Local => Ok(build_local(dispatcher)),
      EventBusConfig::JetStream(config) => build_jetstream(redis, dispatcher, config).await,
    }
  }
}

fn build_dispatcher(
  realtime: Arc<Realtime>,
) -> Result<Arc<EventDispatcher>, HandlerRegistrationError> {
  let mut dispatcher = EventDispatcher::new();

  listeners::register(&mut dispatcher)?;
  register_event_handlers(&mut dispatcher, realtime)?;

  Ok(Arc::new(dispatcher))
}

fn build_local(dispatcher: Arc<EventDispatcher>) -> EventBusRuntime {
  let event_bus = Arc::new(EventBus::local(dispatcher));

  EventBusRuntime::local(event_bus)
}

/// todo надо бы сделать эз этого полноценный Builder
async fn build_jetstream(
  redis: Arc<RedisClient>,
  dispatcher: Arc<EventDispatcher>,
  config: JetStreamEventBusConfig,
) -> Result<EventBusRuntime, EventBusProviderError> {
  let JetStreamEventBusConfig {
    nats: nats_config,
    stream,
    consumer,
    subjects,
    dedup,
    processor,
    incoming,
  } = config;

  let nats = Arc::new(NatsClient::connect(&nats_config).await?);

  nats.ensure_stream(&stream).await?;

  let subscription = nats.subscribe(&consumer).await?;

  let topology_health = nats_client::health::HealthCheck::new(Arc::clone(&nats), stream, consumer);

  let publisher = Arc::new(JetStreamEventPublisher::new(Arc::clone(&nats), subjects));

  let event_bus = Arc::new(EventBus::with_distributed_publisher(
    Arc::clone(&dispatcher),
    publisher,
  ));

  let dedup_store = Arc::new(RedisDedupStore::new(redis, dedup));

  let processor = Arc::new(IncomingEventProcessor::new(
    dispatcher,
    dedup_store,
    processor,
  ));

  let worker = JetStreamIncomingConsumer::new(processor, subscription, incoming);
  let consumer_health = worker.health_check();

  Ok(EventBusRuntime::jetstream(
    event_bus,
    worker,
    topology_health,
    consumer_health,
  ))
}
