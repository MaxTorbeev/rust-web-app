use std::sync::Arc;

use crate::app::config::HttpConfig;
use auth::AuthConfig;
use event_bus::{EventBus, EventBusError};
use realtime::{register_event_handlers, Realtime, RealtimeConfig};
use redis_client::{RedisClient, RedisConfig};

mod config;
mod http;
mod listeners;
mod state;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let redis = Arc::new(RedisClient::connect(&RedisConfig::default()).await?);

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

fn build_event_bus(
    realtime: Arc<Realtime>,
) -> Result<Arc<EventBus>, EventBusError> {
    let mut event_bus = EventBus::new();

    listeners::register(&mut event_bus)?;
    register_event_handlers(&mut event_bus, realtime)?;

    Ok(Arc::new(event_bus))
}
