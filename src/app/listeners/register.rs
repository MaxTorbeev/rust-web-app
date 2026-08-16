use event_bus::{EventBus, EventBusError};

use super::websocket_connected;
use super::websocket_disconnected;

pub fn register(event_bus: &mut EventBus) -> Result<(), EventBusError> {
  websocket_connected::register(event_bus)?;
  websocket_disconnected::register(event_bus)?;

  Ok(())
}
