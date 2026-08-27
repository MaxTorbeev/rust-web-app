pub struct AuthConfig {
  pub login: String,
  pub password_hash: String,
}

impl AuthConfig {
  pub fn from_env() -> Result<Self, std::env::VarError> {
    let login = std::env::var("APP_AUTH_LOGIN")?;
    let password_hash = std::env::var("APP_AUTH_PASSWORD_HASH")?;

    Ok(Self {
      login,
      password_hash,
    })
  }
}
