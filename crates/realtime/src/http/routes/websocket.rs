use crate::requests::websocket_query::WebSocketQuery;
use crate::websocket::handle_socket;
use crate::{Realtime, RealtimeAccess};
use api_response::ApiError;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use event_bus::EventBus;
use std::sync::Arc;

pub async fn websocket(
  ws: WebSocketUpgrade,
  Query(query): Query<WebSocketQuery>,
  State(event_bus): State<Arc<EventBus>>,
  State(realtime): State<Arc<Realtime>>,
) -> Result<Response, ApiError> {
  let RealtimeAccess { application, token } = realtime
    .verify_access_token(&query.access_token)
    .map_err(|_| ApiError::unauthorized("Invalid access token"))?;

  let connection = application.create_connection(token);

  Ok(
    ws.on_upgrade(|socket| async move {
      handle_socket(socket, connection, application, event_bus).await
    })
    .into_response(),
  )
}
