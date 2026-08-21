use event_bus::{EventBusError, EventDispatcher};

use super::websocket_connected;
use super::websocket_disconnected;

pub fn register(dispatcher: &mut EventDispatcher) -> Result<(), EventBusError> {
  websocket_connected::register(dispatcher)?;
  websocket_disconnected::register(dispatcher)?;

  Ok(())
}
