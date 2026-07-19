use crate::requests::websocket_query::WebSocketQuery;
use crate::websocket::handle_socket;
use api_response::ApiError;
use auth::SessionStore;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use std::sync::Arc;
use crate::Connection;

pub async fn websocket(
    ws: WebSocketUpgrade,
    Query(query): Query<WebSocketQuery>,
    State(session): State<Arc<SessionStore>>,
) -> Result<Response, ApiError> {
    let session = session
        .find(query.token.as_str())
        .await
        .map_err(|_e| ApiError::unauthorized("Invalid token"))?;

    let connection = Connection::new(session.user);

    Ok(ws
        .on_upgrade(|_socket| async move { handle_socket(_socket, connection).await })
        .into_response())
}
