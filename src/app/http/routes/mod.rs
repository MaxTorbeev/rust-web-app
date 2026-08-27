//! Public HTTP routes for the HLS switcher.
//!
mod health;

use crate::app::state::AppState;
use auth::{check, login};
use axum::routing::post;
use axum::{Router, routing::get};
use realtime::routes::{access_token, broadcast_message, websocket};

pub fn init(state: AppState) -> Router {
  Router::new()
    .route("/auth/realtime/{application_id}/token", post(access_token))
    .route("/auth/login", post(login))
    .route("/auth/check", get(check))
    .route("/", get(websocket))
    .route("/channels/{channel}/messages", post(broadcast_message))
    .with_state(state)
}
