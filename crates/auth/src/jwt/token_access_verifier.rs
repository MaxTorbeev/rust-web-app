use crate::{TokenCapability, TokenClaims, TokenVerifyError, VerifiedToken};
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{
  Algorithm, DecodingKey, Validation, decode, decode_header, get_current_timestamp,
};

pub struct TokenAccessVerifier {
  key_name: String,
  decoding_key: DecodingKey,
  validation: Validation,
}

impl TokenAccessVerifier {
  pub fn new(key_name: impl Into<String>, key_secret: impl AsRef<[u8]>) -> Self {
    let mut validation = Validation::new(Algorithm::HS256);

    validation.leeway = 60;

    // TODO(security): WARNING: JWT is not bound to an expected issuer or audience.
    // Add and validate product/application claims when the credential model supports them.
    validation.validate_aud = false;

    Self {
      key_name: key_name.into(),
      decoding_key: DecodingKey::from_secret(key_secret.as_ref()),
      validation,
    }
  }

  pub fn unverified_key_id(token: &str) -> Result<String, TokenVerifyError> {
    let header = decode_header(token).map_err(TokenVerifyError::InvalidToken)?;

    header.kid.ok_or(TokenVerifyError::MissingKeyId)
  }

  pub fn verify(&self, token: &str) -> Result<VerifiedToken, TokenVerifyError> {
    let key_id = Self::unverified_key_id(token)?;

    if key_id != self.key_name {
      return Err(TokenVerifyError::UnexpectedKeyId {
        expected: self.key_name.clone(),
        actual: key_id,
      });
    }

    let token_data =
      decode::<TokenClaims>(token, &self.decoding_key, &self.validation).map_err(|error| {
        if matches!(error.kind(), ErrorKind::ExpiredSignature) {
          TokenVerifyError::Expired
        } else {
          TokenVerifyError::InvalidToken(error)
        }
      })?;

    let claims = token_data.claims;

    if claims.client_id.as_deref() == Some("") {
      return Err(TokenVerifyError::EmptyClientId);
    }

    let now = get_current_timestamp();

    if claims.iat > now.saturating_add(60) {
      return Err(TokenVerifyError::IssuedAtInFuture {
        issued_at: claims.iat,
        now,
      });
    }

    // TODO(security): WARNING: parsing only validates the capability JSON shape.
    // Every protocol action must also enforce the requested operation and channel.
    let capability = claims.capability.parse::<TokenCapability>()?;

    Ok(VerifiedToken {
      client_id: claims.client_id,
      issued_at: claims.iat,
      expires_at: claims.exp,
      capability,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use jsonwebtoken::{EncodingKey, Header, encode};

  const KEY_NAME: &str = "primary";
  const KEY_SECRET: &[u8] = b"secret";
  const CAPABILITY: &str = r#"{"private:*":["subscribe","publish"]}"#;

  fn valid_claims(now: u64, client_id: Option<&str>) -> TokenClaims {
    TokenClaims {
      iat: now,
      exp: now + 3600,
      client_id: client_id.map(str::to_owned),
      capability: CAPABILITY.to_owned(),
    }
  }

  fn encode_token(claims: &TokenClaims, key_name: Option<&str>, key_secret: &[u8]) -> String {
    let mut header = Header::new(Algorithm::HS256);
    header.kid = key_name.map(str::to_owned);

    encode(&header, claims, &EncodingKey::from_secret(key_secret))
      .expect("test JWT must be encoded")
  }

  fn verify(token: &str) -> Result<VerifiedToken, TokenVerifyError> {
    TokenAccessVerifier::new(KEY_NAME, KEY_SECRET).verify(token)
  }

  /// Проверяет чтение identity, временных claims и capability из корректного JWT.
  #[test]
  fn verifies_valid_token() {
    let now = get_current_timestamp();
    let token = encode_token(
      &valid_claims(now, Some("client-123")),
      Some(KEY_NAME),
      KEY_SECRET,
    );
    let verified = verify(&token).expect("valid JWT must be accepted");

    assert_eq!(verified.client_id.as_deref(), Some("client-123"));
    assert_eq!(verified.issued_at, now);
    assert_eq!(verified.expires_at, now + 3600);

    let operations = verified.capability.resources().get("private:*").unwrap();

    assert!(operations.contains("subscribe"));
    assert!(operations.contains("publish"));
  }

  /// Проверяет, что строка без структуры JWT отклоняется как невалидный токен.
  #[test]
  fn rejects_malformed_token() {
    assert!(matches!(
      verify("not-a-jwt"),
      Err(TokenVerifyError::InvalidToken(_)),
    ));
  }

  /// Проверяет, что JWT с подписью от другого секрета не проходит проверку.
  #[test]
  fn rejects_token_with_invalid_signature() {
    let now = get_current_timestamp();
    let token = encode_token(
      &valid_claims(now, Some("client-123")),
      Some(KEY_NAME),
      b"another-secret",
    );

    assert!(matches!(
      verify(&token),
      Err(TokenVerifyError::InvalidToken(_)),
    ));
  }

  /// Проверяет, что JWT без идентификатора ключа `kid` отклоняется до проверки подписи.
  #[test]
  fn rejects_token_without_key_id() {
    let now = get_current_timestamp();
    let token = encode_token(&valid_claims(now, Some("client-123")), None, KEY_SECRET);

    assert!(matches!(
      verify(&token),
      Err(TokenVerifyError::MissingKeyId),
    ));
  }

  /// Проверяет, что JWT, подписанный неизвестным `kid`, возвращает оба значения ключа.
  #[test]
  fn rejects_unexpected_key_id() {
    let now = get_current_timestamp();
    let token = encode_token(
      &valid_claims(now, Some("client-123")),
      Some("secondary"),
      KEY_SECRET,
    );

    assert!(matches!(
      verify(&token),
      Err(TokenVerifyError::UnexpectedKeyId { expected, actual })
        if expected == KEY_NAME && actual == "secondary",
    ));
  }

  /// Проверяет, что токен старше настроенного 60-секундного leeway считается просроченным.
  #[test]
  fn rejects_expired_token() {
    let now = get_current_timestamp();
    let mut claims = valid_claims(now, Some("client-123"));
    claims.iat = now.saturating_sub(3600);
    claims.exp = now.saturating_sub(120);

    let token = encode_token(&claims, Some(KEY_NAME), KEY_SECRET);

    assert!(matches!(verify(&token), Err(TokenVerifyError::Expired)));
  }

  /// Проверяет, что `iat` значительно позже серверного времени отклоняется явно.
  #[test]
  fn rejects_token_issued_in_future() {
    let now = get_current_timestamp();
    let mut claims = valid_claims(now, Some("client-123"));
    claims.iat = now + 120;

    let token = encode_token(&claims, Some(KEY_NAME), KEY_SECRET);

    assert!(matches!(
      verify(&token),
      Err(TokenVerifyError::IssuedAtInFuture { issued_at, .. })
        if issued_at == now + 120,
    ));
  }

  /// Проверяет, что присутствующий, но пустой `client_id` не принимается как identity.
  #[test]
  fn rejects_empty_client_id() {
    let now = get_current_timestamp();
    let token = encode_token(&valid_claims(now, Some("")), Some(KEY_NAME), KEY_SECRET);

    assert!(matches!(
      verify(&token),
      Err(TokenVerifyError::EmptyClientId),
    ));
  }

  /// Проверяет поддерживаемый Ably-сценарий JWT без привязанного `client_id`.
  #[test]
  fn accepts_missing_client_id() {
    let now = get_current_timestamp();
    let token = encode_token(&valid_claims(now, None), Some(KEY_NAME), KEY_SECRET);

    let verified = verify(&token).expect("JWT without client_id must be accepted");

    assert_eq!(verified.client_id, None);
  }

  /// Проверяет, что capability должна содержать корректный JSON с разрешениями.
  #[test]
  fn rejects_invalid_capability_json() {
    let now = get_current_timestamp();
    let mut claims = valid_claims(now, Some("client-123"));
    claims.capability = "not-json".to_owned();

    let token = encode_token(&claims, Some(KEY_NAME), KEY_SECRET);

    assert!(matches!(
      verify(&token),
      Err(TokenVerifyError::InvalidCapability(_)),
    ));
  }
}
