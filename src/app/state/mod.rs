use std::sync::Arc;
use axum::extract::FromRef;
use auth::{AuthConfig, SessionStore};
use event_bus::EventBus;
use realtime::ChannelHub;
use realtime::presence_hub::PresenceHub;
use redis_client::{RedisClient};

#[derive(Clone)]
pub struct AppState {
    pub redis: Arc<RedisClient>,
    pub auth: Arc<AuthConfig>,
    pub sessions: Arc<SessionStore>,
    pub channel_hub: Arc<ChannelHub>,
    pub presence_hub: Arc<PresenceHub>,
    pub event_bus: Arc<EventBus>
}

impl AppState {
    pub fn new(redis: Arc<RedisClient>, auth: Arc<AuthConfig>, event_bus: Arc<EventBus>) -> Self {
        let sessions = Arc::new(SessionStore::new(redis.clone()));
        let channel_hub = Arc::new(ChannelHub::new());
        let presence_hub = Arc::new(PresenceHub::new());

        Self {
            redis,
            sessions,
            auth,
            channel_hub,
            event_bus,
            presence_hub
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

impl FromRef<AppState> for Arc<ChannelHub> {
    fn from_ref(state: &AppState) -> Self {
        state.channel_hub.clone()
    }
}

impl FromRef<AppState> for Arc<EventBus> {
    fn from_ref(state: &AppState) -> Self {
        state.event_bus.clone()
    }
}