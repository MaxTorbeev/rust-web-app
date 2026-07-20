use std::sync::Arc;
use axum::extract::FromRef;
use auth::{AuthConfig, SessionStore};
use realtime::ChannelHub;
use redis_client::{RedisClient};

#[derive(Clone)]
pub struct AppState {
    pub redis: Arc<RedisClient>,
    pub auth: Arc<AuthConfig>,
    pub sessions: Arc<SessionStore>,
    pub channel_hub: Arc<ChannelHub>,
}

impl AppState {
    pub fn new(redis: Arc<RedisClient>, auth: Arc<AuthConfig>) -> Self {
        let sessions = Arc::new(SessionStore::new(redis.clone()));
        let channel_hub = Arc::new(ChannelHub::new());

        Self {
            redis,
            sessions,
            auth,
            channel_hub,
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