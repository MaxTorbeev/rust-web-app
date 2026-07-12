use std::sync::Arc;
use auth::AuthConfig;
use redis_client::MultiplexedConnection;

#[derive(Clone)]
pub struct AppState {
    pub redis: MultiplexedConnection,
    pub auth: Arc<AuthConfig>
}
