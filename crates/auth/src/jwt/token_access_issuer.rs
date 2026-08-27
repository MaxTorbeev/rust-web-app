use crate::{TokenCapability, TokenClaims, TokenIssueError};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode, get_current_timestamp};

pub struct TokenAccessIssuer {
  key_name: String,
  encoding_key: EncodingKey,
}

impl TokenAccessIssuer {
  pub fn new(key_name: impl Into<String>, key_secret: impl AsRef<[u8]>) -> Self {
    Self {
      key_name: key_name.into(),
      encoding_key: EncodingKey::from_secret(key_secret.as_ref()),
    }
  }

  pub fn issue(
    &self,
    client_id: Option<String>,
    capability: &TokenCapability,
    ttl_seconds: u64,
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

    encode(&header, &claims, &self.encoding_key).map_err(TokenIssueError::TokenEncoding)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::TokenAccessVerifier;
  use jsonwebtoken::decode_header;

  const KEY_NAME: &str = "primary";
  const KEY_SECRET: &[u8] = b"secret";

  fn capability() -> TokenCapability {
    r#"{"private:chat":["publish","subscribe"]}"#
      .parse()
      .expect("test capability must be valid")
  }

  /// Проверяет полный цикл выпуска и проверки JWT вместе с `kid`, TTL и capability.
  #[test]
  fn issued_token_passes_verification() {
    const TTL_SECONDS: u64 = 3600;

    let token = TokenAccessIssuer::new(KEY_NAME, KEY_SECRET)
      .issue(Some("client-123".to_owned()), &capability(), TTL_SECONDS)
      .expect("valid claims must produce a JWT");

    let header = decode_header(&token).expect("issued JWT must have a valid header");
    let verified = TokenAccessVerifier::new(KEY_NAME, KEY_SECRET)
      .verify(&token)
      .expect("issued JWT must pass verification");

    assert_eq!(header.alg, Algorithm::HS256);
    assert_eq!(header.kid.as_deref(), Some(KEY_NAME));
    assert_eq!(verified.client_id.as_deref(), Some("client-123"));
    assert_eq!(verified.expires_at - verified.issued_at, TTL_SECONDS);

    let operations = verified
      .capability
      .resources()
      .get("private:chat")
      .expect("issued JWT must preserve its capability resource");

    assert!(operations.contains("publish"));
    assert!(operations.contains("subscribe"));
  }

  /// Проверяет, что issuer не создаёт JWT с присутствующим, но пустым `client_id`.
  #[test]
  fn rejects_empty_client_id() {
    let result =
      TokenAccessIssuer::new(KEY_NAME, KEY_SECRET).issue(Some(String::new()), &capability(), 3600);

    assert!(matches!(result, Err(TokenIssueError::EmptyClientId)));
  }
}
