use std::time::Duration;
use axum::extract::ws::{WebSocket, Message};
use futures_util::SinkExt;
use futures_util::stream::SplitSink;
use tokio::sync::mpsc::Receiver;
use tokio::task::{JoinError, JoinHandle};
use crate::{PreparedFrame};
use crate::transport::{SessionError, ShutdownListener, WriterPolicy};

type WriterJoinResult = Result<Result<(), axum::Error>, JoinError>;

pub struct WebSocketWriter {
  task: JoinHandle<Result<(), axum::Error>>,
}

impl WebSocketWriter {

  /// Spawn new web socket writer event loop task
  ///
  /// Ответ будет:
  /// * Ok(Ok(())) — writer штатно завершился;
  /// * Ok(Err(error)) — ошибка записи в WebSocket;
  /// * Err(join_error) — task отменён или запаниковал.
  pub(crate) fn spawn(
    mut receiver: Receiver<PreparedFrame>,
    mut sender: SplitSink<WebSocket, Message>,
  ) -> Self {
    let task = tokio::spawn(async move {
      while let Some(frame) = receiver.recv().await {
        sender
          .send(frame.into_websocket_message())
          .await?;
      }

      // Очередь закрыта и полностью обработана.
      Ok(())
    });

    Self { task }
  }

  pub(crate) async fn finish(
    &mut self,
    policy: WriterPolicy,
    shutdown: &mut ShutdownListener
  ) -> Result<(), SessionError> {
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
  pub(crate) async fn wait_until_stopped(&mut self) -> Result<(), SessionError> {
    Self::map_join_result((&mut self.task).await)
  }

  async fn drain_until_shutdown(
    &mut self,
    shutdown: &mut ShutdownListener,
  ) -> Result<(), SessionError> {
    tokio::select! {
      writer_result = &mut self.task => {
        Self::map_join_result(writer_result)
      }

      true = shutdown.requested() => {
        self.abort_and_wait().await
      }

      _ = tokio::time::sleep(Duration::from_secs(2)) => {
        // Writer не успел дописать очередь за отведённое время.
        // Принудительно останавливаем task, чтобы session не зависла.
        self.abort_and_wait().await?;

        Err(SessionError::WriterDrainTimedOut)
      }
    }
  }

  pub fn abort(&mut self) {
    self.task.abort()
  }

  async fn abort_and_wait(&mut self) -> Result<(), SessionError> {
    self.task.abort();

    match (&mut self.task).await {
      Err(error) if error.is_cancelled() => Ok(()),
      result => Self::map_join_result(result),
    }
  }

  fn map_join_result(result: WriterJoinResult) -> Result<(), SessionError> {
    match result {
      // Writer task завершилась штатно после обработки очереди.
      Ok(Ok(())) => Ok(()),

      // Task завершилась без panic, но запись в WebSocket вернула ошибку.
      Ok(Err(error)) => {
        Err(SessionError::Write(error))
      }

      // Writer task была отменена или завершилась panic.
      Err(error) => {
        Err(SessionError::WriterTaskFailed(error))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::future::pending;
  use std::time::Duration;
  use tokio::time::timeout;
  use crate::transport::shutdown_channel;

  fn pending_writer() -> WebSocketWriter {
    WebSocketWriter {
      task: tokio::spawn(
        pending::<Result<(), axum::Error>>()
      ),
    }
  }

  async fn panic_writer_task() -> Result<(), axum::Error> {
    panic!("intentional writer task panic")
  }

  #[test]
  fn maps_websocket_write_error() {
    let write_error = axum::Error::new(
      std::io::Error::other("websocket write failed")
    );

    let result = WebSocketWriter::map_join_result(
      Ok(Err(write_error))
    );

    assert!(matches!(result, Err(SessionError::Write(_))));
  }

  #[tokio::test(flavor = "current_thread")]
  async fn maps_writer_task_panic() {
    let join_result = tokio::spawn(panic_writer_task()).await;

    assert!(matches!(
      WebSocketWriter::map_join_result(join_result),
      Err(SessionError::WriterTaskFailed(error)) if error.is_panic(),
    ));
  }

  #[tokio::test(flavor = "current_thread")]
  async fn drain_aborts_pending_writer_after_shutdown_request() {
    // A forever-pending task models a writer blocked by socket backpressure.
    let mut writer = pending_writer();

    let (shutdown_trigger, mut shutdown_listener) = shutdown_channel();

    let result = {
      let finish = writer.finish(
        WriterPolicy::DrainUntilShutdown,
        &mut shutdown_listener,
      );
      tokio::pin!(finish);

      // Drain must keep waiting while neither the writer nor shutdown is ready.
      tokio::select! {
        biased;

        result = &mut finish => {
          panic!("drain finished before shutdown: {result:?}");
        }

        _ = tokio::task::yield_now() => {}
      }

      shutdown_trigger.request();

      timeout(Duration::from_secs(1), &mut finish)
      .await
      .expect("shutdown must stop a blocked writer")
    };

    assert!(result.is_ok(), "an expected writer abort is not an error");
    assert!(writer.task.is_finished(), "the blocked writer must be stopped");
  }

  #[tokio::test(flavor = "current_thread", start_paused = true)]
  async fn drain_aborts_pending_writer_after_timeout() {
    // A pending writer must not keep the WebSocket session alive indefinitely.
    let mut writer = pending_writer();
    let (_shutdown_trigger, mut shutdown_listener) = shutdown_channel();

    let result = writer
      .finish(
        WriterPolicy::DrainUntilShutdown,
        &mut shutdown_listener,
      )
      .await;

    assert!(matches!(
      result,
      Err(SessionError::WriterDrainTimedOut),
    ));
    assert!(writer.task.is_finished(), "the timed-out writer must be stopped");
  }
}
