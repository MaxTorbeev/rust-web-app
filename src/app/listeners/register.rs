use std::sync::Arc;
use event_bus::{EventBus, EventBusError};

use super::websocket_connected;
use super::websocket_disconnected;

pub async fn register(event_bus: Arc<EventBus>) -> Result<(), EventBusError> {
  websocket_connected::register(event_bus.clone()).await?;
  websocket_disconnected::register(event_bus.clone()).await?;

  Ok(())
}