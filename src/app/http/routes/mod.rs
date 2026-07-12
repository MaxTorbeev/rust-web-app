//! Public HTTP routes for the HLS switcher.

use axum::{routing::get, Router};
use crate::app::state::AppState;

mod handlers;

pub fn init(state: AppState) -> Router {
    Router::new()
        .route("/health/live", get(handlers::live))
        .route("/health/redis", get(handlers::redis))
        .with_state(state)
}
