use std::sync::Arc;
use axum::extract::ws::WebSocket;
use futures_util::StreamExt;
use tokio::sync::{mpsc};
use tokio::sync::mpsc::Receiver;
use crate::{Connection, OutboundSendError, OutboundSender, PreparedFrame, ProtocolMessage, RealtimeApplication};
use crate::transport::{shutdown_channel, EndReason, Heartbeat, ShutdownListener, WebSocketWriter, WriterPolicy, SessionError};
use crate::transport::protocol_reader::ProtocolReader;

const OUTBOUND_QUEUE_CAPACITY: usize = 128;

pub struct WebSocketSession {
  sender: OutboundSender,
  connection: Connection,
  application: Arc<RealtimeApplication>,
  shutdown_listener: ShutdownListener,
  reader: ProtocolReader,
  writer: WebSocketWriter,
  heartbeat: Option<Heartbeat>,
}

impl WebSocketSession {
  pub fn new(
    socket: WebSocket,
    connection: Connection,
    application: Arc<RealtimeApplication>,
  ) -> Self {
    let (
      websocket_sender,
      websocket_receiver,
    ) = socket.split();

    let (
      sender,
      receiver,
      shutdown_listener
    ) = Self::outbound_channel();

    let reader = ProtocolReader::new(websocket_receiver);
    let writer = WebSocketWriter::spawn(receiver, websocket_sender);

    Self {
      connection,
      application,
      sender,
      shutdown_listener,
      reader,
      writer,
      heartbeat: None,
    }
  }

  /// Start websocket session
  pub async fn run(mut self) -> Result<(), SessionError> {
    let end_reason = match self.send_connected() {
      Ok(()) => {
        self.heartbeat = Some(Heartbeat::spawn(&self.sender, &self.application));

        self.wait_for_end().await
      },
      Err(e) => EndReason::ProtocolFailed(e)
    };

    self.finish(end_reason).await
  }

  /// Finish web socket session
  pub async fn finish(mut self, reason: EndReason) -> Result<(), SessionError> {
    let writer_policy = reason.writer_policy();

    // Finish heartbeat task
    if let Some(heartbeat) = self.heartbeat {
      heartbeat.finish().await;
    }

    // Slow writer should be stopped before cleanup
    if matches!(writer_policy, WriterPolicy::Abort) {
      self.writer.abort();
    }

    // Удаляем соединение из всех channels и presence и прочее.
    self.application.disconnect_connection(&self.connection.id).await;

    // Очистить структуру отправителя
    drop(self.sender);

    let writer_result = self
      .writer
      .finish(writer_policy, &mut self.shutdown_listener)
      .await;

    // Проверить на ошибки, что бы не упустить ошибку writer
    match reason.into_result() {
      Ok(_) => writer_result,
      Err(session_error) => {
        // Если writer уже завершился с собственной ошибкой,
        // логируем её, но возвращаем первичную ошибку session.
        if let Err(e) = writer_result {
          tracing::error!(?e, "writer also failed while websocket session was stopping");
        }

        Err(session_error)
      }
    }
  }

  fn outbound_channel() -> (OutboundSender, Receiver<PreparedFrame>, ShutdownListener) {
    let (shutdown_trigger, shutdown_listener) = shutdown_channel();

    let (
      queue_sender,
      queue_receiver
    ) = mpsc::channel::<PreparedFrame>(OUTBOUND_QUEUE_CAPACITY);

    let sender = OutboundSender::new(queue_sender, shutdown_trigger);

    (
      sender,
      queue_receiver,
      shutdown_listener,
    )
  }

  /// Отправить сообщение о том, что пользователь успешно соединился
  fn send_connected(&self) -> Result<(), OutboundSendError> {
    let connected = ProtocolMessage::connected(&self.connection);

    self.sender.try_enqueue_protocol_message(&connected)
  }

  /// Waiting for ending session
  async fn wait_for_end(&mut self) -> EndReason {
    // TODO(security): WARNING: authorization.expires_at is not observed after the handshake.
    // Trigger reauthorization before expiry and close the connection if renewal fails.
    tokio::select! {
      biased; // Директива указывающая фиксированный порядок веток

      // При нормальном Disconnect очередь можно дописать.
      // При ошибке writer нужно остановить принудительно.
      reader_result = self.reader.run(
        &self.sender,
        &self.connection,
        self.application.as_ref()
      ) => reader_result.into(),

      true = self.shutdown_listener.requested() => {
        EndReason::ShutdownRequested
      }

      writer_result = self.writer.wait_until_stopped() => {
        EndReason::WriterStopped(writer_result)
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::Duration;
  use tokio::time::timeout;

  #[tokio::test(flavor = "current_thread")]
  async fn outbound_channel_is_bounded_and_propagates_shutdown() {
    let (sender, _receiver, mut shutdown_listener) =
      WebSocketSession::outbound_channel();

    // The session queue must reject new frames once all bounded slots are used.
    for _ in 0..OUTBOUND_QUEUE_CAPACITY {
      sender
        .try_enqueue_protocol_message(&ProtocolMessage::heartbeat())
        .expect("frame must fit within the configured capacity");
    }

    assert!(matches!(
      sender.try_enqueue_protocol_message(&ProtocolMessage::heartbeat()),
      Err(OutboundSendError::QueueFull),
    ));

    // The signal is state-based: requesting it before awaiting must not lose it.
    sender.request_shutdown();
    sender.request_shutdown();

    let requested = timeout(
      Duration::from_secs(1),
      shutdown_listener.requested(),
    )
      .await
      .expect("shutdown listener must observe the sender signal");

    assert!(requested);
  }
}
