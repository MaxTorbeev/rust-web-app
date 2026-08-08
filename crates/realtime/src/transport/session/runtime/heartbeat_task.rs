use std::time::Duration;
use tokio::task::JoinHandle;
use crate::{OutboundSender, ProtocolMessage, RealtimeApplication};

pub struct Heartbeat {
  task: JoinHandle<()>,
}

impl Heartbeat {

  /// Spawn new heartbeat event loop task
  pub fn spawn(sender: &OutboundSender, app: &RealtimeApplication) -> Self {
    let interval = Duration::from_millis(
      app.settings.max_idle_interval
    );

    let task = tokio::spawn({
      let sender = sender.clone();

      async move {
        loop {
          tokio::time::sleep(interval).await;

          let heartbeat = ProtocolMessage::heartbeat();

          if sender.try_enqueue_protocol_message(&heartbeat).is_err() {
            sender.request_shutdown();

            break;
          }
        }
      }
    });

    Self { task }
  }

  pub async fn finish(self) {
    self.task.abort();

    match self.task.await {
      Ok(()) => {}
      Err(e) if e.is_cancelled()  => {}
      Err(e) => {
        tracing::error!("Heartbeat task failed: {}", e);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use auth::{TokenAccessIssuer, TokenAccessVerifier};
  use tokio::sync::mpsc;
  use tokio::time::timeout;
  use crate::{ApplicationId, ApplicationSettings};
  use crate::transport::shutdown_channel;

  #[tokio::test(flavor = "current_thread")]
  async fn sends_heartbeat_within_max_idle_interval() {
    let mut application = RealtimeApplication::new(
      ApplicationId::new("application-1"),
      TokenAccessIssuer::new("test-key", b"test-secret"),
      TokenAccessVerifier::new("test-key", b"test-secret"),
    );

    application.settings = ApplicationSettings {
      max_idle_interval: 5,
      ..ApplicationSettings::default()
    };

    let (shutdown_trigger, _shutdown_listener) = shutdown_channel();
    let (queue_sender, mut queue_receiver) = mpsc::channel(1);
    let sender = OutboundSender::new(queue_sender, shutdown_trigger);
    let heartbeat = Heartbeat::spawn(&sender, &application);

    let received = timeout(
      Duration::from_millis(100),
      queue_receiver.recv(),
    ).await;

    heartbeat.finish().await;

    assert!(
      matches!(received, Ok(Some(_))),
      "heartbeat must be sent within max_idle_interval",
    );
  }
}
