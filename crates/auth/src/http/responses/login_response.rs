use serde::Serialize;
use crate::UserIdentity;

#[derive(Serialize)]
pub struct LoginResponse {
  pub token_type: String,
  pub access_token: String,
  pub user: UserIdentity,
}

impl LoginResponse {
  pub fn new(access_token: String, user: UserIdentity) -> Self {
    Self {
      token_type: "Bearer".to_string(),
      access_token,
      user
    }
  }
}