use std::sync::Arc;
use axum::extract::FromRef;
use auth::{AuthConfig, SessionStore};
use redis_client::{RedisClient};

#[derive(Clone)]
pub struct AppState {
    pub redis: Arc<RedisClient>,
    pub auth: Arc<AuthConfig>,
    pub sessions: Arc<SessionStore>
}

impl AppState {
    pub fn new(redis: Arc<RedisClient>, auth: Arc<AuthConfig>) -> Self {
        let sessions = Arc::new(SessionStore::new(redis.clone()));

        Self {
            redis,
            sessions,
            auth,
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