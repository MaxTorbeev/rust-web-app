use event_bus::{EventBusError, EventDispatcher};
use realtime::{WebsocketDisconnected};

pub fn register(dispatcher: &mut EventDispatcher) -> Result<(), EventBusError> {
  dispatcher.register(|event: WebsocketDisconnected| async move {
    tracing::info!("websocket disconnected: {:?}", event.connection_id);

    Ok(())
  })?;

  Ok(())
}
