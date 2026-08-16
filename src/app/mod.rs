use std::sync::Arc;
use auth::AuthConfig;
use event_bus::EventBus;
use realtime::{Realtime, RealtimeConfig};
use crate::app::config::{HttpConfig};
use redis_client::{RedisClient, RedisConfig};

mod http;
mod state;
mod config;

mod listeners;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let redis = Arc::new(RedisClient::connect(&RedisConfig::default()).await?);

    let auth = Arc::new(AuthConfig::from_env()?);

    let mut event_bus = EventBus::new();

    listeners::register(&mut event_bus)?;

    let event_bus = Arc::new(event_bus);

    let realtime_config = RealtimeConfig::from_env();

    let realtime = Arc::new(Realtime::from_config(realtime_config?));

    let app_state = state::AppState::new(
        redis,
        auth,
        event_bus,
        realtime
    );

    let routes = http::routes::init(app_state);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(HttpConfig::from_env()?.url).await?;

    axum::serve(listener, routes).await?;

    Ok(())
}
