use std::sync::Arc;

use event_bus::{EventDispatcher, HandlerRegistrationError};

use crate::Realtime;

mod channel_message_submitted;
mod presence_channel_changed;

/// Registers the required local handlers for realtime domain events.
pub fn register_event_handlers(
  dispatcher: &mut EventDispatcher,
  realtime: Arc<Realtime>,
) -> Result<(), HandlerRegistrationError> {
  channel_message_submitted::register(dispatcher, realtime.clone())?;
  presence_channel_changed::register(dispatcher, realtime)
}
