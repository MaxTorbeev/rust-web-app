use std::sync::Arc;

use crate::app::config::HttpConfig;
use crate::app::providers::EventBusProvider;
use auth::AuthConfig;
use realtime::{Realtime, RealtimeConfig};
use redis_client::{RedisClient, RedisConfig};
use support::{
  BootGeneration, DeploymentSlot, NodeId, NodeIdentity, NodeInstance, app::read_env,
  timestamp::Timestamp,
};

mod config;
mod health;
mod http;
mod listeners;
mod providers;
mod state;
mod version;

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let app_version = version::AppVersion::CURRENT;
  let node_instance = NodeInstance::new(
    NodeId::try_new(read_env("APP_NODE_ID")?)?,
    BootGeneration::generate(),
    Timestamp::now(),
  );

  let node_identity = NodeIdentity::new(node_instance.node_id.clone(), DeploymentSlot::from_env()?);

  let redis_config = RedisConfig::from_env()?;
  let redis = Arc::new(RedisClient::connect(&redis_config).await?);

  let redis_health = redis_client::health::HealthCheck::new(Arc::clone(&redis));

  tracing::info!(
    version = app_version.version(),
    revision = app_version.revision(),
    "application starting"
  );

  let auth = Arc::new(AuthConfig::from_env()?);

  let realtime_config = RealtimeConfig::from_env()?;
  let presence_runtime = match std::env::var("PRESENCE_STORE_DRIVER").as_deref() {
    Err(std::env::VarError::NotPresent) | Ok("memory") => None,
    Ok("redis") => Some(
      realtime::redis::RedisPresenceRuntime::claim(
        redis.clone(),
        realtime::redis::RedisKeys::new(&read_env("APP")?, &read_env("APP_ENV")?)?,
        &node_instance,
        realtime::PresenceLedgerPolicy::from_settings(&realtime::ApplicationSettings::default()),
      )
      .await?,
    ),
    _ => return Err("PRESENCE_STORE_DRIVER must be memory or redis".into()),
  };
  let realtime = Arc::new(match &presence_runtime {
    Some(runtime) => Realtime::from_redis(realtime_config, runtime.store()),
    None => Realtime::from_config(realtime_config, node_instance),
  });

  let event_bus_runtime =
    EventBusProvider::build(Arc::clone(&redis), Arc::clone(&realtime)).await?;

  if presence_runtime.is_some() && !event_bus_runtime.is_distributed() {
    return Err("Redis Presence requires EVENT_BUS_DRIVER=nats".into());
  }
  let event_bus = event_bus_runtime.event_bus();
  let event_bus_health = event_bus_runtime.health_check();
  let health = Arc::new(health::HealthCheck::new(
    app_version,
    node_identity,
    redis_health,
    event_bus_health,
    realtime.clone(),
  ));

  let app_state = state::AppState::new(redis, auth, event_bus.clone(), realtime.clone(), health);

  let routes = http::routes::init(app_state);

  // run our app with hyper, listening globally on port 3000
  let listener = tokio::net::TcpListener::bind(HttpConfig::from_env()?.url).await?;

  let http_server = async move { axum::serve(listener, routes).await };

  let presence_worker = async {
    match presence_runtime {
      Some(runtime) => runtime.run(event_bus).await,
      None => std::future::pending().await,
    }
  };
  let result: Result<(), Box<dyn std::error::Error + Send + Sync>> = tokio::select! {
    result = http_server => result.map_err(Into::into),
    result = event_bus_runtime.run() => result.map_err(Into::into),
    result = presence_worker => result,
    result = tokio::signal::ctrl_c() => result.map_err(Into::into),
  };
  realtime.shutdown().await;
  result
}
