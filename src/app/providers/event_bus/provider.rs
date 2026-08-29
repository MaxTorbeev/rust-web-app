use std::sync::Arc;
use realtime::Realtime;
use redis_client::RedisClient;
use crate::app::providers::{EventBusProviderError, EventBusRuntime};

pub struct EventBusProvider;

impl EventBusProvider {
  pub async fn build(redis: Arc<RedisClient>, realtime: Arc<Realtime>) -> Result<EventBusRuntime, EventBusProviderError> {
    todo!()
  }
}