use std::sync::Arc;

use event_bus::{EventBus, EventBusError};

use crate::Realtime;

mod channel_message_published;

/// Registers the required local handlers for realtime domain events.
pub fn register_event_handlers(
  event_bus: &mut EventBus,
  realtime: Arc<Realtime>,
) -> Result<(), EventBusError> {
  channel_message_published::register(event_bus, realtime)
}
