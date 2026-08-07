use axum::extract::ws::{WebSocket, Message};
use futures_util::SinkExt;
use futures_util::stream::SplitSink;
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinError;
use crate::{OutboundSendError, OutboundSender, PreparedFrame};
use crate::transport::{ShutdownListener, WriterPolicy};

pub struct WebSocketWriter {
  task: tokio::task::JoinHandle<()>,
}

impl WebSocketWriter {
  /// Spawn new web socket writer event loop task
  pub(crate) fn spawn(
    mut receiver: Receiver<PreparedFrame>,
    mut sender: SplitSink<WebSocket, Message>,
  ) -> Self {
    let task = tokio::task::spawn(async move {
      while let Some(frame) = receiver.recv().await {
        if let Err(error) = sender
          .send(frame.into_websocket_message())
          .await
        {
          tracing::error!(%error, "websocket write failed");
          break;
        }
      }
    });

    Self { task }
  }

  pub(crate) async fn finish(
    &mut self,
    policy: WriterPolicy,
    shutdown: &mut ShutdownListener
  ) -> Result<(), OutboundSendError> {
    match policy {
      WriterPolicy::DrainUntilShutdown => {
        self.drain_until_shutdown(shutdown).await
      }

      WriterPolicy::Abort => {
        self.abort_and_wait().await
      }

      WriterPolicy::AlreadyStopped => Ok(())
    }
  }

  /// Ждать пока Writer не остановится
  pub(crate) async fn wait_until_stopped(&mut self) -> Result<(), JoinError> {
    (&mut self.task).await
  }

  async fn drain_until_shutdown(
    &mut self,
    shutdown: &mut ShutdownListener,
  ) -> Result<(), OutboundSendError> {
      tokio::select! {
        writer_result = &mut self.task => {
          Self::handle_join_result(writer_result)
        }

        true = shutdown.requested() => self.abort_and_wait().await
      }
  }

  pub fn abort(&mut self) {
    self.task.abort()
  }

  async fn abort_and_wait(&mut self) -> Result<(), OutboundSendError> {
    self.task.abort();

    match (&mut self.task).await {
      Ok(()) => Ok(()),
      Err(e) if e.is_cancelled()  => Ok(()),
      Err(e) => {
        tracing::error!("Heartbeat task failed: {}", e);
        Err(OutboundSendError::QueueClosed)
      }
    }
  }

  fn handle_join_result(
    result: Result<(), JoinError>,
  ) -> Result<(), OutboundSendError> {
    match result {
      Ok(()) => Ok(()),
      Err(e) => {
        tracing::error!("websocket writer task failed: {}", e);
        Err(OutboundSendError::QueueClosed)
      }
    }
  }
}