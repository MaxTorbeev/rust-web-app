use crate::app::config::TlsConfig;

pub struct HttpConfig {
    pub url: String,
    pub tls: Option<TlsConfig>
}

impl HttpConfig {
    pub fn from_env() -> Result<Self, std::env::VarError> {
        let tls_enabled = std::env::var("APP_TLS_ENABLED")
          .is_ok_and(|value| value == "true" || value == "1");

        let url = std::env::var("APP_URL").unwrap_or_else(|error| {
            tracing::error!(%error, "app url env var is missing or invalid");

            "0.0.0.0:4008".to_string()
        }).to_string();

        let tls = if tls_enabled {
            Some(TlsConfig {
                cert_path: std::env::var("APP_TLS_CERT")?,
                key_path: std::env::var("APP_TLS_KEY")?,
            })
        } else {
            None
        };

        Ok(Self {url, tls})
    }
}