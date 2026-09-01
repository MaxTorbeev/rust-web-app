use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use support::health::{HealthReport, VerifyHealth};

use crate::app::health::HealthCheck;
use crate::app::http::responses::ReadyHealthResponse;

use super::health_response;

pub(crate) async fn ready(State(health): State<Arc<HealthCheck>>) -> Response {
  let state = health.verify().await;
  let status = if state.is_healthy() {
    StatusCode::OK
  } else {
    StatusCode::SERVICE_UNAVAILABLE
  };

  health_response(status, ReadyHealthResponse::from(&state))
}
