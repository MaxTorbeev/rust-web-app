use serde::Deserialize;
use crate::TokenCapability;

#[derive(Debug, Deserialize)]
pub struct TokenClaims {
  pub iat: Option<u64>,
  pub exp: u64,
  pub client_id: String,
  pub capability: TokenCapability,
}