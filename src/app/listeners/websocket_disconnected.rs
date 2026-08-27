use event_bus::{EventDispatcher, HandlerRegistrationError};
use realtime::WebsocketDisconnected;

pub fn register(dispatcher: &mut EventDispatcher) -> Result<(), HandlerRegistrationError> {
  dispatcher.register(|event: WebsocketDisconnected| async move {
    tracing::info!("websocket disconnected: {:?}", event.connection_id);

    Ok(())
  })?;

  Ok(())
}
