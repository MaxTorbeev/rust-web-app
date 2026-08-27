use event_bus::{EventDispatcher, HandlerRegistrationError};

use super::websocket_connected;
use super::websocket_disconnected;

pub fn register(dispatcher: &mut EventDispatcher) -> Result<(), HandlerRegistrationError> {
  websocket_connected::register(dispatcher)?;
  websocket_disconnected::register(dispatcher)?;

  Ok(())
}
