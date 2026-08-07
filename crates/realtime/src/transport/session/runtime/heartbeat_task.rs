use std::time::Duration;
use tokio::task::JoinHandle;
use crate::{OutboundSender, ProtocolMessage};

pub struct Heartbeat {
  task: JoinHandle<()>,
}

impl Heartbeat {

  /// Spawn new heartbeat event loop task
  pub fn spawn(sender: &OutboundSender) -> Self {
    let task = tokio::spawn({
      let sender = sender.clone();

      async move {
        loop {
          tokio::time::sleep(Duration::from_millis(10_000)).await;

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