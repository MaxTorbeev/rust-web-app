use std::sync::Arc;
use event_bus::{EventBus, EventDispatcher};
use realtime::{register_event_handlers, Realtime};
use redis_client::RedisClient;
use crate::app::listeners;
use crate::app::providers::{EventBusProviderError, EventBusRuntime};

pub struct EventBusProvider;

impl EventBusProvider {
  pub async fn build(
    redis: Arc<RedisClient>,
    realtime: Arc<Realtime>
  ) -> Result<EventBusRuntime, EventBusProviderError> {
    let mut dispatcher = EventDispatcher::new();

    listeners::register(&mut dispatcher)?;

    register_event_handlers(&mut dispatcher, realtime)?;

    let event_bus = Arc::new(EventBus::local(Arc::new(dispatcher)));

    Ok(EventBusRuntime::local(event_bus))
  }
}