use crate::AuthConfig;
use crate::password::PasswordHasher;

pub struct Authenticator {

}

impl Authenticator {
  pub fn is_verify(config: &AuthConfig, login: &str, password: &str) -> bool {
    let _ = config.login.to_string();

    if login == config.login.to_string() {
      return PasswordHasher::verify(password, config.password_hash.to_string().as_ref())
    }

    false
  }
}