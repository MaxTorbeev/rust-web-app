use event_bus::{EventBusError, EventDispatcher};
use realtime::WebsocketConnected;

pub fn register(dispatcher: &mut EventDispatcher) -> Result<(), EventBusError> {
  dispatcher.register(|event: WebsocketConnected| async move {
    tracing::info!("websocket connected received event: {:?}", event.connection_id);

    Ok(())
  })?;

  Ok(())
}
