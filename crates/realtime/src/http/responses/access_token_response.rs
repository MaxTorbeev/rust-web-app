use serde::Serialize;

#[derive(Serialize)]
pub struct AccessTokenResponse {
  pub application_id: String,
  pub client_id: String,
  pub jwt: String,
  pub ttl_seconds: u64,
}
