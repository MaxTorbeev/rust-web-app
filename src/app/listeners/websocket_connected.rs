use std::sync::Arc;
use event_bus::{EventBus, EventBusError};
use realtime::WebsocketConnected;

pub async fn register(event_bus: Arc<EventBus>) -> Result<(), EventBusError> {
  event_bus.listen(|event: WebsocketConnected| async move {
    tracing::info!("websocket connected received event: {:?}", event.connection_id);
  }).await?;

  Ok(())
}