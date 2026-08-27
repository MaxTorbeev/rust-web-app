use std::sync::Arc;

use event_bus::{EventDispatcher, HandlerError, HandlerRegistrationError};
use thiserror::Error;

use crate::{ChannelMessageSubmitted, ProtocolMessage, Realtime};

#[derive(Debug, Error)]
#[error("realtime application `{application_id}` is not registered")]
struct UnknownApplication {
  application_id: String,
}

pub(super) fn register(
  dispatcher: &mut EventDispatcher,
  realtime: Arc<Realtime>,
) -> Result<(), HandlerRegistrationError> {
  dispatcher.register(move |event: ChannelMessageSubmitted| {
    // Получаем Arc<RealtimeApplication> до async move,
    // чтобы не клонировать Arc<Realtime> для каждого сообщения.
    let application = realtime.application(&event.application_id);

    async move {
      let application = application.ok_or_else(|| {
        let err = UnknownApplication {
          application_id: event.application_id.as_str().to_owned(),
        };

        // Registry формируется при запуске и не изменится от повторной
        // доставки того же события в этот процесс.
        HandlerError::permanent(err)
      })?;

      application
        .channel_hub
        .broadcast(
          &event.channel,
          ProtocolMessage::message(&event.channel, event.messages),
        )
        .await
        // BroadcastError сейчас означает детерминированную ошибку
        // сериализации frame: повтор с тем же payload её не исправит.
        .map_err(HandlerError::permanent)?;

      Ok(())
    }
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{ApplicationId, RealtimeConfig};
  use event_bus::{DispatchError, Event, EventMessage, ProcessingErrorClass};

  fn test_realtime() -> Arc<Realtime> {
    Arc::new(Realtime::from_config(RealtimeConfig {
      application_id: ApplicationId::new("application-1"),
      key_name: "application-1.test-key".to_owned(),
      key_secret: "test-secret".to_owned(),
    }))
  }

  fn message(application_id: &str) -> EventMessage {
    EventMessage::try_from_event(&ChannelMessageSubmitted {
      application_id: ApplicationId::new(application_id),
      channel: "test-channel".to_owned(),
      messages: Vec::new(),
    })
    .expect("event message must encode")
  }

  #[tokio::test(flavor = "current_thread")]
  async fn dispatch_without_local_recipients_succeeds() {
    let mut dispatcher = EventDispatcher::new();
    register(&mut dispatcher, test_realtime()).expect("handler must register");

    dispatcher
      .dispatch(&message("application-1"))
      .await
      .expect("no local recipients is a successful dispatch");
  }

  #[tokio::test(flavor = "current_thread")]
  async fn dispatch_reports_unknown_application_as_handler_error() {
    let mut dispatcher = EventDispatcher::new();
    register(&mut dispatcher, test_realtime()).expect("handler must register");

    let result = dispatcher.dispatch(&message("application-2")).await;

    match result {
      Err(DispatchError::Handler { event_name, source }) => {
        assert_eq!(event_name, ChannelMessageSubmitted::NAME);
        assert_eq!(source.class(), ProcessingErrorClass::Permanent,);
        assert!(source.to_string().contains("application-2"));
      }
      other => panic!("expected handler error, got {other:?}"),
    }
  }
}
