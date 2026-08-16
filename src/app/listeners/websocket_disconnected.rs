use event_bus::{EventBus, EventBusError};
use realtime::{WebsocketDisconnected};

pub fn register(event_bus: &mut EventBus) -> Result<(), EventBusError> {
  event_bus.register(|event: WebsocketDisconnected| async move {
    tracing::info!("websocket diconnected: {:?}", event.connection_id);

    Ok(())
  })?;

  Ok(())
}
