use serde::Serialize;

#[derive(Serialize)]
pub struct AccessTokenResponse {
  pub token_type: String,
  pub access_token: String,
}

impl AccessTokenResponse {
  pub fn new(access_token: String) -> Self {
    Self {
      token_type:  "Bearer".to_string(),
      access_token
    }
  }
}