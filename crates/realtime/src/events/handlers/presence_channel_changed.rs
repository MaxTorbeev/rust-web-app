use crate::{PresenceChannelChanged, Realtime};
use event_bus::{EventDispatcher, HandlerError, HandlerRegistrationError};
use std::sync::Arc;

pub(super) fn register(
  dispatcher: &mut EventDispatcher,
  realtime: Arc<Realtime>,
) -> Result<(), HandlerRegistrationError> {
  dispatcher.register(move |event: PresenceChannelChanged| {
    let application = realtime.application(&event.channel.application_id);
    async move {
      let application = application.ok_or_else(|| {
        HandlerError::permanent(std::io::Error::other(
          "Presence event refers to an unknown application",
        ))
      })?;
      application
        .presence()
        .project(application.router(), &event)
        .await
        .map_err(|error| {
          // Не продолжаем обслуживать Presence с недостоверной локальной проекцией.
          application.stop_presence();
          HandlerError::retryable(error)
        })
    }
  })
}
