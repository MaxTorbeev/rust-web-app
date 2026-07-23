use std::sync::Arc;
use auth::AuthConfig;
use event_bus::EventBus;
use crate::app::config::{HttpConfig};
use redis_client::{RedisClient, RedisConfig};

mod http;
mod state;
mod config;

mod listeners;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let redis = Arc::new(RedisClient::connect(&RedisConfig::default()).await?);

    let auth = Arc::new(AuthConfig::from_env()?);

    let event_bus = Arc::new(EventBus::new().await?);

    listeners::register(event_bus.clone()).await?;

    let app_state = state::AppState::new(redis, auth, event_bus);

    let routes = http::routes::init(app_state);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(HttpConfig::from_env().unwrap().url).await?;

    axum::serve(listener, routes).await?;

    Ok(())
}
