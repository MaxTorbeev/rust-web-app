use jsonwebtoken::{decode_header, Algorithm, DecodingKey, Validation, decode, get_current_timestamp};
use jsonwebtoken::errors::ErrorKind;
use crate::{TokenCapability, TokenClaims, TokenVerifyError, VerifiedToken};

pub struct TokenAccessVerifier {
  key_name: String,
  decoding_key: DecodingKey,
  validation: Validation,
}

impl TokenAccessVerifier {
  pub fn new(
    key_name: impl Into<String>,
    key_secret: impl AsRef<[u8]>
  ) -> Self {
    let mut validation = Validation::new(Algorithm::HS256);

    validation.leeway = 60;

    validation.validate_aud = false;

    Self {
      key_name: key_name.into(),
      decoding_key: DecodingKey::from_secret(key_secret.as_ref()),
      validation
    }
  }

  pub fn verify(&self, token: &str) -> Result<VerifiedToken, TokenVerifyError> {
    let header = decode_header(token)
      .map_err(TokenVerifyError::InvalidToken)?;

    let key_id = header
      .kid
      .ok_or(TokenVerifyError::MissingKeyId)?;

    if key_id != self.key_name {
      return Err(TokenVerifyError::UnexpectedKeyId {
        expected: self.key_name.clone(),
        actual: key_id
      });
    }

    let token_data = decode::<TokenClaims>(token, &self.decoding_key, &self.validation)
      .map_err(|error| {
        if matches!(error.kind(), ErrorKind::ExpiredSignature) {
          TokenVerifyError::Expired
        } else {
          TokenVerifyError::InvalidToken(error)
        }
      })?;

    let claims = token_data.claims;

    if claims.client_id.as_deref() == Some("") {
      return Err(TokenVerifyError::EmptyClientId)
    }

    let now = get_current_timestamp();

    if claims.iat > now.saturating_add(60) {
      return Err(TokenVerifyError::IssuedAtInFuture {
        issued_at: claims.iat,
        now
      });
    }

    let capability = claims
      .capability
      .parse::<TokenCapability>()?;

    Ok(VerifiedToken {
      client_id: claims.client_id,
      issued_at: claims.iat,
      expires_at: claims.exp,
      capability
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use jsonwebtoken::{encode, EncodingKey, Header};

  #[test]
  fn verifies_valid_token() {
    const KEY_NAME: &str = "primary";
    const KEY_SECRET: &[u8] = b"secret";

    let now = get_current_timestamp();
    let claims = serde_json::json!({
      "iat": now,
      "exp": now + 3600,
      "x-ably-clientId": "client-123",
      "x-ably-capability": r#"{"private:*":["subscribe","publish"]}"#
    });

    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some(KEY_NAME.to_string());

    let token = encode(
      &header,
      &claims,
      &EncodingKey::from_secret(KEY_SECRET)
    ).unwrap();

    let verified = TokenAccessVerifier::new(KEY_NAME, KEY_SECRET)
      .verify(&token)
      .unwrap();

    assert_eq!(verified.client_id.as_deref(), Some("client-123"));
    assert_eq!(verified.issued_at, now);
    assert_eq!(verified.expires_at, now + 3600);

    let operations = verified
      .capability
      .resources()
      .get("private:*")
      .unwrap();

    assert!(operations.contains("subscribe"));
    assert!(operations.contains("publish"));
  }

}
