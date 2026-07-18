//! Public HTTP routes for the HLS switcher.

use crate::app::state::AppState;
use axum::routing::post;
use axum::{Router, routing::get};
use auth::{check, login};

mod handlers;

pub fn init(state: AppState) -> Router {
    Router::new()
        .route("/health/live", get(handlers::live))
        .route("/health/redis", get(handlers::redis))
        .route("/auth/login", post(login))
        .route("/auth/check", get(check))
        .with_state(state)
}
