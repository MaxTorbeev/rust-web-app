use event_bus::{EventDispatcher, HandlerRegistrationError};
use realtime::WebsocketConnected;

pub fn register(dispatcher: &mut EventDispatcher) -> Result<(), HandlerRegistrationError> {
  dispatcher.register(|event: WebsocketConnected| async move {
    tracing::info!("websocket connected received event: {:?}", event.connection_id);

    Ok(())
  })?;

  Ok(())
}
