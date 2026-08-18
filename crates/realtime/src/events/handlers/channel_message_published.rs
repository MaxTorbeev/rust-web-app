use std::sync::Arc;

use event_bus::{Event, EventBus, EventBusError};
use thiserror::Error;

use crate::{ChannelMessagePublished, ProtocolMessage, Realtime};

#[derive(Debug, Error)]
#[error("realtime application `{application_id}` is not registered")]
struct UnknownApplication {
  application_id: String,
}

pub(super) fn register(
  event_bus: &mut EventBus,
  realtime: Arc<Realtime>,
) -> Result<(), EventBusError> {
  event_bus.register(move |event: ChannelMessagePublished| {
    // Получаем Arc<RealtimeApplication> до async move,
    // чтобы не клонировать Arc<Realtime> для каждого сообщения.
    let application = realtime.application(&event.application_id);

    async move {
      let application = application.ok_or_else(|| {
        let err = UnknownApplication {
          application_id: event.application_id.as_str().to_owned(),
        };

        EventBusError::handler(ChannelMessagePublished::NAME, err)
      })?;

      application
        .channel_hub
        .broadcast(
          &event.channel,
          ProtocolMessage::message(&event.channel, event.messages),
        )
        .await
        .map_err(|error| {
          EventBusError::handler(ChannelMessagePublished::NAME, error)
        })?;

      Ok(())
    }
  })
}
