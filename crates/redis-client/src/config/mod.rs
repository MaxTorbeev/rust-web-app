
pub struct RedisConfig {
    pub host: String,
    pub port: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub db: String,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            username: std::env::var("REDIS_USERNAME").ok(),
            password: std::env::var("REDIS_PASSWORD").ok(),
            host: std::env::var("REDIS_HOST")
                .unwrap_or_else(|error| {
                    tracing::error!(%error, "redis url env var is missing or invalid");

                    "127.0.0.1".to_string()
                })
                .to_string(),
            port: std::env::var("REDIS_PORT").unwrap_or_default().to_string(),
            db: 0.to_string(),
        }
    }
}

impl RedisConfig {
    pub fn to_url(&self) -> String {
        let auth = match (self.username.as_ref(), self.password.as_ref()) {
            (Some(username), Some(password)) => format!("{username}:{password}"),
            (None, Some(password)) => format!(":{password}"),
            _ => String::new(),
        };

        format!("redis://{}@{}:{}/{}", auth, self.host, self.port, self.db)
    }
}
