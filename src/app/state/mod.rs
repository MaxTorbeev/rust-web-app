use std::sync::Arc;
use axum::extract::FromRef;
use auth::AuthConfig;
use redis_client::MultiplexedConnection;

#[derive(Clone)]
pub struct AppState {
    pub redis: MultiplexedConnection,
    pub auth: Arc<AuthConfig>
}

impl FromRef<AppState> for Arc<AuthConfig> {
    fn from_ref(state: &AppState) -> Self {
        state.auth.clone()
    }
}