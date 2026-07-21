use std::sync::Arc;
use event_bus::{EventBus, EventBusError};
use realtime::{WebsocketDisconnected};

pub async fn register(event_bus: Arc<EventBus>) -> Result<(), EventBusError> {
  event_bus.listen(|event: WebsocketDisconnected| async move {
    tracing::info!("websocket diconnected: {:?}", event.connection_id);
  }).await?;

  Ok(())
}