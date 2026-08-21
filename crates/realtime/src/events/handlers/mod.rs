use std::sync::Arc;

use event_bus::{EventBusError, EventDispatcher};

use crate::Realtime;

mod channel_message_published;

/// Registers the required local handlers for realtime domain events.
pub fn register_event_handlers(
  dispatcher: &mut EventDispatcher,
  realtime: Arc<Realtime>,
) -> Result<(), EventBusError> {
  channel_message_published::register(dispatcher, realtime)
}
