use serde::Deserialize;

#[derive(Deserialize)]
pub struct LoginRequest {
  pub login: String,
  pub password: String,
}