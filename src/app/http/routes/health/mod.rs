mod live;
mod ready;

use api_response::ApiResponse;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;

pub(super) use live::live;
pub(super) use ready::ready;

fn health_response<T>(status: StatusCode, resource: T) -> Response
where
  T: Serialize,
{
  (
    status,
    [(header::CACHE_CONTROL, "no-store")],
    ApiResponse::new(resource),
  )
    .into_response()
}
