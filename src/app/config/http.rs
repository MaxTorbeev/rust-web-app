pub struct HttpConfig {
    pub url: String,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("APP_URL").unwrap_or_else(|error| {
                tracing::error!(%error, "app url env var is missing or invalid");

                "0.0.0.0:4008".to_string()
            }).to_string(),
        }
    }
}
