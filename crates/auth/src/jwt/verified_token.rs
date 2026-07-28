use crate::TokenCapability;

pub struct VerifiedToken {
  pub client_id: Option<String>,
  pub issued_at: u64,
  pub expires_at: u64,
  pub capability: TokenCapability,
}