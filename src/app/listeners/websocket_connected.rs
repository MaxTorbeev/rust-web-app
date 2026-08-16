use event_bus::{EventBus, EventBusError};
use realtime::WebsocketConnected;

pub fn register(event_bus: &mut EventBus) -> Result<(), EventBusError> {
  event_bus.register(|event: WebsocketConnected| async move {
    tracing::info!("websocket connected received event: {:?}", event.connection_id);

    Ok(())
  })?;

  Ok(())
}
