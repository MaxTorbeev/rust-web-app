use std::sync::Arc;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::header;
use api_response::ApiError;
use support::decode_to_string;
use crate::{ApplicationKeyName, Realtime, RealtimeApplication};

pub struct BroadcastApplication(
  pub Arc<RealtimeApplication>
);

impl<S> FromRequestParts<S> for BroadcastApplication
where
  S: Send + Sync,
  Arc<Realtime>: FromRef<S>,
{
  type Rejection = ApiError;

  async fn from_request_parts(
    parts: &mut axum::http::request::Parts,
    state: &S,
  ) -> Result<Self, Self::Rejection> {
    let authorization = parts
      .headers
      .get(header::AUTHORIZATION)
      .and_then(|value| value.to_str().ok())
      .ok_or_else(|| ApiError::unauthorized("Missing Basic credentials"))?;

    let mut segments = authorization.split_ascii_whitespace();

    let scheme = segments.next();
    let encoded = segments.next();
    let extra = segments.next();

    let encoded = match (scheme, encoded, extra) {
      (Some(scheme), Some(encoded), None)
      if scheme.eq_ignore_ascii_case("Basic") => encoded,

      _ => {
        return Err(
          ApiError::unauthorized("Invalid Basic credentials")
        );
      }
    };

    let decoded = decode_to_string(encoded)
      .map_err(|_| ApiError::unauthorized("Invalid Basic credentials"))?;

    let (key_name, _key_secret) = decoded
      .split_once(':')
      .ok_or_else(|| ApiError::unauthorized("Invalid Basic credentials"))?;

    // TODO(security): WARNING: Basic auth currently selects an application by id only.
    // Validate the full product key, secret, active/revoked state and publish permissions.
    let application_key_name = key_name
      .parse::<ApplicationKeyName>()
      .map_err(|_| ApiError::unauthorized("Invalid Basic credentials"))?;

    let realtime = Arc::<Realtime>::from_ref(state);

    let application = realtime
      .application(application_key_name.application_id())
      .ok_or_else(|| ApiError::unauthorized("Unknown application"))?;

    Ok(Self(application))
  }
}
