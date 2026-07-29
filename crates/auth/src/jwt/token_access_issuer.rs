use jsonwebtoken::{get_current_timestamp, EncodingKey, Header, encode, Algorithm};
use crate::{TokenCapability, TokenClaims, TokenIssueError};

pub struct TokenAccessIssuer {
  key_name: String,
  encoding_key: EncodingKey
}

impl TokenAccessIssuer {
  pub fn new(
    key_name: impl Into<String>,
    key_secret: impl AsRef<[u8]>
  ) -> Self {
    Self {
      key_name: key_name.into(),
      encoding_key: EncodingKey::from_secret(key_secret.as_ref())
    }
  }

  pub fn issue(
    &self,
    client_id: Option<String>,
    capability: &TokenCapability,
    ttl_seconds: u64
  ) -> Result<String, TokenIssueError> {
    if client_id.as_deref() == Some("") {
      return Err(TokenIssueError::EmptyClientId);
    }

    let now = get_current_timestamp();

    let claims = TokenClaims {
      iat: now,
      exp: now.saturating_add(ttl_seconds),
      client_id,
      capability: serde_json::to_string(capability)?,
    };

    let mut header = Header::new(Algorithm::HS256);

    header.kid = Some(self.key_name.clone());

    encode(
      &header,
      &claims,
      &self.encoding_key,
    ).map_err(TokenIssueError::TokenEncoding)
  }
}