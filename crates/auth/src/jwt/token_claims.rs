use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TokenClaims {
  pub iat: u64,
  pub exp: u64,
  #[serde(rename = "x-ably-clientId")]
  pub client_id: Option<String>,
  #[serde(rename = "x-ably-capability")]
  pub capability: String,
}