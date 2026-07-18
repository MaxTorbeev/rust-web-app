use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct UserIdentity {
  pub login: String,
}

impl UserIdentity {
  pub fn new(login: String) -> Self {
    Self { login }
  }
}