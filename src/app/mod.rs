use std::sync::Arc;

use crate::app::config::HttpConfig;
use auth::AuthConfig;
use realtime::{Realtime, RealtimeConfig};
use redis_client::{RedisClient, RedisConfig};
use crate::app::providers::EventBusProvider;

mod config;
mod health;
mod http;
mod listeners;
mod state;
mod providers;
mod version;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
  let app_version = version::AppVersion::CURRENT;

  let redis_config = RedisConfig::from_env()?;
  let redis = Arc::new(RedisClient::connect(&redis_config).await?);

  let redis_health = redis_client::health::HealthCheck::new(Arc::clone(&redis));
  let health = Arc::new(health::HealthCheck::new(app_version, redis_health));

  tracing::info!(
    version = health.version().version(),
    revision = health.version().revision(),
    "application starting"
  );

  let auth = Arc::new(AuthConfig::from_env()?);

  let realtime = Arc::new(Realtime::from_config(RealtimeConfig::from_env()?));

  let event_bus_runtime = EventBusProvider::build(
    Arc::clone(&redis),
    Arc::clone(&realtime),
  ).await?;

  let event_bus = event_bus_runtime.event_bus();

  let app_state = state::AppState::new(redis, auth, event_bus, realtime, health);

  let routes = http::routes::init(app_state);

  // run our app with hyper, listening globally on port 3000
  let listener = tokio::net::TcpListener::bind(HttpConfig::from_env()?.url).await?;

  let http_server = async move {
    axum::serve(listener, routes).await
  };

  tokio::select! {
    result = http_server => {
      result?;
    }

    result = event_bus_runtime.run() => {
      result?;
    }
  }

  Ok(())
}
