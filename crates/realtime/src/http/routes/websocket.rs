use crate::requests::websocket_query::WebSocketQuery;
use crate::websocket::handle_socket;
use api_response::ApiError;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use std::sync::Arc;
use event_bus::EventBus;
use crate::{Connection, Realtime, RealtimeAccess};

pub async fn websocket(
    ws: WebSocketUpgrade,
    Query(query): Query<WebSocketQuery>,
    State(event_bus): State<Arc<EventBus>>,
    State(realtime): State<Arc<Realtime>>,
) -> Result<Response, ApiError> {
    let RealtimeAccess {
        application,
        token,
    } = realtime
      .verify_access_token(&query.access_token)
      .map_err(|_| {
          ApiError::unauthorized("Invalid access token")
      })?;

    let connection = Connection::new(token);

    Ok(ws
        .on_upgrade(|_socket| async move {
            handle_socket(_socket, connection, application, event_bus).await
        })
        .into_response())
}
