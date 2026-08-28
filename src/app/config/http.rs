use support::app::{ReadEnvError, read_env_or};

pub struct HttpConfig {
  pub url: String,
}

impl HttpConfig {
  pub fn from_env() -> Result<Self, ReadEnvError> {
    let url = read_env_or("APP_URL", "0.0.0.0:4008")?;

    Ok(Self { url })
  }
}
