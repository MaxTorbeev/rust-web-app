use std::sync::Arc;
use event_bus::{EventBus, EventBusError};

use super::websocket_connected;

pub async fn register(event_bus: Arc<EventBus>) -> Result<(), EventBusError> {
  websocket_connected::register(event_bus.clone()).await?;

  Ok(())
}