//! Public HTTP routes for the HLS switcher.

use crate::app::state::AppState;
use axum::routing::post;
use axum::{Router, routing::get};
use auth::{check, login};
use realtime::routes::websocket;

pub fn init(state: AppState) -> Router {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/check", get(check))
        .route("/", get(websocket))
        .with_state(state)
}
