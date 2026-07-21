use crate::requests::websocket_query::WebSocketQuery;
use crate::websocket::handle_socket;
use api_response::ApiError;
use auth::SessionStore;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use std::sync::Arc;
use event_bus::EventBus;
use crate::{ChannelHub, Connection, WebsocketConnected};

pub async fn websocket(
    ws: WebSocketUpgrade,
    Query(query): Query<WebSocketQuery>,
    State(session): State<Arc<SessionStore>>,
    State(event_bus): State<Arc<EventBus>>,
    State(channel_hub): State<Arc<ChannelHub>>,
) -> Result<Response, ApiError> {
    let session = session
        .find(query.token.as_str())
        .await
        .map_err(|_e| ApiError::unauthorized("Invalid token"))?;

    let connection = Connection::new(session.user);

    if let Err(_e) = event_bus.publish(WebsocketConnected {
        connection_id: connection.id.as_str().to_string(),
    }).await {
        tracing::error!("Failed to publish websocket connection");
    }

    Ok(ws
        .on_upgrade(|_socket| async move {
            handle_socket(_socket, connection, channel_hub).await
        })
        .into_response())
}
