use api_response::ApiError;
use axum::extract::FromRequestParts;
use axum::http::header;
use axum::http::request::Parts;

pub struct BearerToken(String);

impl BearerToken {
  pub fn as_str(&self) -> &str {
    &self.0
  }

  pub fn into_inner(self) -> String {
    self.0
  }
}

impl<S> FromRequestParts<S> for BearerToken
where
  S: Send + Sync,
{
  type Rejection = ApiError;

  async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
    let authorization = parts
      .headers
      .get(header::AUTHORIZATION)
      .and_then(|value| value.to_str().ok())
      .ok_or_else(|| ApiError::unauthorized("Missing Bearer token"))?;

    let mut segments = authorization.split_ascii_whitespace();

    let scheme = segments.next();
    let token = segments.next();
    let extra = segments.next();

    match (scheme, token, extra) {
      (Some(scheme), Some(token), None)
        if scheme.eq_ignore_ascii_case("Bearer") && !token.is_empty() =>
      {
        Ok(Self(token.to_owned()))
      }

      _ => Err(ApiError::unauthorized("Invalid Bearer token")),
    }
  }
}
