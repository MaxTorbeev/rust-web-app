use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use crate::app::health::HealthCheck;
use crate::app::http::responses::LiveHealthResponse;

use super::health_response;

pub(crate) async fn live(State(health): State<Arc<HealthCheck>>) -> Response {
  health_response(StatusCode::OK, LiveHealthResponse::from(health.as_ref()))
}
