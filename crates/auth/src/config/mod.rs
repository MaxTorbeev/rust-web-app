use support::app::{ReadEnvError, read_env};

pub struct AuthConfig {
  pub login: String,
  pub password_hash: String,
}

impl AuthConfig {
  pub fn from_env() -> Result<Self, ReadEnvError> {
    let login = read_env("APP_AUTH_LOGIN")?;
    let password_hash = read_env("APP_AUTH_PASSWORD_HASH")?;

    Ok(Self {
      login,
      password_hash,
    })
  }
}
