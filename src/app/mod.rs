use std::sync::Arc;

use crate::app::config::HttpConfig;
use auth::AuthConfig;
use event_bus::{EventBus, EventDispatcher, HandlerRegistrationError};
use realtime::{Realtime, RealtimeConfig, register_event_handlers};
use redis_client::{RedisClient, RedisConfig};
use crate::app::providers::EventBusProvider;

mod config;
mod http;
mod listeners;
mod state;
mod providers;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
  let redis_config = RedisConfig::from_env()?;
  let redis = Arc::new(RedisClient::connect(&redis_config).await?);

  let auth = Arc::new(AuthConfig::from_env()?);

  let realtime = Arc::new(Realtime::from_config(RealtimeConfig::from_env()?));

  let event_bus = build_event_bus(Arc::clone(&realtime))?;

  let app_state = state::AppState::new(redis, auth, event_bus, realtime);

  let routes = http::routes::init(app_state);

  // run our app with hyper, listening globally on port 3000
  let listener = tokio::net::TcpListener::bind(HttpConfig::from_env()?.url).await?;

  axum::serve(listener, routes).await?;

  Ok(())
}

fn build_event_bus(realtime: Arc<Realtime>) -> Result<Arc<EventBus>, HandlerRegistrationError> {
  let mut dispatcher = EventDispatcher::new();

  listeners::register(&mut dispatcher)?;
  register_event_handlers(&mut dispatcher, realtime)?;

  Ok(Arc::new(EventBus::local(Arc::new(dispatcher))))
}
