use auth::{AuthConfig, SessionStore};
use axum::extract::FromRef;
use event_bus::EventBus;
use realtime::Realtime;
use redis_client::RedisClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
  pub redis: Arc<RedisClient>,
  pub auth: Arc<AuthConfig>,
  pub sessions: Arc<SessionStore>,
  pub event_bus: Arc<EventBus>,
  pub realtime: Arc<Realtime>,
}

impl AppState {
  pub fn new(
    redis: Arc<RedisClient>,
    auth: Arc<AuthConfig>,
    event_bus: Arc<EventBus>,
    realtime: Arc<Realtime>,
  ) -> Self {
    let sessions = Arc::new(SessionStore::new(redis.clone()));

    Self {
      redis,
      sessions,
      auth,
      event_bus,
      realtime,
    }
  }
}

impl FromRef<AppState> for Arc<AuthConfig> {
  fn from_ref(state: &AppState) -> Self {
    state.auth.clone()
  }
}

impl FromRef<AppState> for Arc<SessionStore> {
  fn from_ref(state: &AppState) -> Self {
    state.sessions.clone()
  }
}

impl FromRef<AppState> for Arc<EventBus> {
  fn from_ref(state: &AppState) -> Self {
    state.event_bus.clone()
  }
}
impl FromRef<AppState> for Arc<Realtime> {
  fn from_ref(state: &AppState) -> Self {
    state.realtime.clone()
  }
}
