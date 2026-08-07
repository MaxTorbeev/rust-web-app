use std::sync::Arc;
use std::time::Duration;
use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::{SplitSink, SplitStream};
use tokio::sync::{mpsc, watch};
use tokio::sync::mpsc::{Receiver};
use tokio::task::JoinHandle;
use event_bus::EventBus;
use crate::{Connection, OutboundSendError, OutboundSender, PreparedFrame, ProtocolMessage, ProtocolOutcome, RealtimeApplication, SessionEndReason, WebsocketConnected, WebsocketDisconnected};
use crate::protocol_handlers::{handle_protocol_message, SocketContext};

const OUTBOUND_QUEUE_CAPACITY: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriterAction {
  /// Штатно завершить writer после отправки всех уже поставленных в очередь кадров.
  Drain,
  /// Немедленно остановить writer, не ожидая отправки оставшихся кадров.
  Abort,
  /// Writer уже завершился и не требует дополнительных действий.
  AlreadyFinished,
}

struct SessionOutcome {
  result: Result<(), OutboundSendError>,
  writer_action: WriterAction,
}

pub async fn handle_socket(
  socket: WebSocket,
  connection: Connection,
  application: Arc<RealtimeApplication>,
  event_bus: Arc<EventBus>,
) {
  emit_connected(&event_bus, &connection).await;

  let result = run_socket_session(socket, &connection, &application).await;

  emit_disconnected(&event_bus, &connection).await;

  if let Err(error) = result {
    tracing::error!(?error, "error occurred while handling socket");
  }
}

async fn run_socket_session(
  socket: WebSocket,
  connection: &Connection,
  application: &RealtimeApplication,
) -> Result<(), OutboundSendError> {
  let (
    outbound_sender,
    receiver,
    mut shutdown_receiver
  ) = outbound_channel();

  let (
    websocket_sender,
    websocket_receiver,
  ) = socket.split();

  let mut writer_task = spawn_writer(receiver, websocket_sender);
  let mut heartbeat_task: Option<JoinHandle<()>> = None;

  let outcome = match send_connected(&outbound_sender, connection) {
    Ok(()) => {
      heartbeat_task = Some(
        spawn_heartbeat(&outbound_sender)
      );

      let session_end_reason = wait_for_session_end(
        connection,
        application,
        &outbound_sender,
        &mut shutdown_receiver,
        websocket_receiver,
        &mut writer_task,
      );

      match session_end_reason.await {
        SessionEndReason::ProtocolLoopFinished(result) => {
          let writer_action = if result.is_ok() {
            WriterAction::Drain
          } else {
            WriterAction::Abort
          };

          SessionOutcome {
            result,
            writer_action,
          }
        }

        SessionEndReason::ShutdownRequested => {
          tracing::debug!(connection_id = connection.id.as_str(), "websocket shutdown requested");

          SessionOutcome {
            result: Ok(()),
            writer_action: WriterAction::Abort,
          }
        }

        SessionEndReason::WriterFinished(writer_result) => {
          if let Err(error) = writer_result {
            tracing::error!(%error, "websocket writer task failed");
          }

          SessionOutcome {
            result: Err(OutboundSendError::QueueClosed),
            writer_action: WriterAction::AlreadyFinished,
          }
        }
      }
    }

    Err(error) => {
      SessionOutcome {
        result: Err(error),
        writer_action: WriterAction::Abort,
      }
    }
  };

  // Больше не создаём heartbeat-сообщения.
  if let Some(heartbeat_task) = heartbeat_task {
    heartbeat_task.abort();
    let _ = heartbeat_task.await;
  }

  let mut writer_was_aborted = matches!(outcome.writer_action, WriterAction::Abort);

  // Медленный клиент не должен дренировать зависший writer.
  if writer_was_aborted {
    writer_task.abort();
  }

  // Удаляем соединение из всех channels и presence.
  cleanup_connection(connection, application).await;

  // Удаляем последний локальный Sender.
  // При Drain writer обработает остаток очереди и завершится.
  drop(outbound_sender);

  // Если writer не завершился внутри select!,
  // даём ему дописать оставшуюся очередь.
  if !matches!(  outcome.writer_action,  WriterAction::AlreadyFinished) {
    let writer_result =
      if matches!(outcome.writer_action, WriterAction::Drain) {
        tokio::select! {
          writer_result = &mut writer_task => writer_result,
          // Пока writer дописывал очередь,
          // другой sender мог обнаружить её переполнение.
          Ok(_) = shutdown_receiver.wait_for(|should_shutdown| *should_shutdown ) => {
            writer_was_aborted = true;
            writer_task.abort();
            writer_task.await
          }
        }
      } else {
        writer_task.await
      };

    if let Err(error) = writer_result {
      let expected_abort = writer_was_aborted && error.is_cancelled();

      if !expected_abort {
        tracing::error!(%error, "websocket writer task failed during shutdown");

        if outcome.result.is_ok() {
          return Err(OutboundSendError::QueueClosed);
        }
      }
    }
  }

  outcome.result
}

/// Запустить сессию и ожидать ее конца
async fn wait_for_session_end(
  connection: &Connection,
  application: &RealtimeApplication,
  outbound_sender: &OutboundSender,
  shutdown_receiver: &mut watch::Receiver<bool>,
  websocket_receiver: SplitStream<WebSocket>,
  writer_task: &mut JoinHandle<()>,
) -> SessionEndReason {
  tokio::select! {
    // При нормальном Disconnect очередь можно дописать.
    // При ошибке writer нужно остановить принудительно.
    protocol_result = run_protocol_loop(
      websocket_receiver,
      outbound_sender,
      connection,
      application
    ) => SessionEndReason::ProtocolLoopFinished(protocol_result),

    _ = shutdown_receiver.wait_for(|should_shutdown| *should_shutdown) => {
       SessionEndReason::ShutdownRequested
    }

    writer_result = writer_task => SessionEndReason::WriterFinished(writer_result)
  }
}

async fn run_protocol_loop(
  mut websocket_receiver: SplitStream<WebSocket>,
  outbound_sender: &OutboundSender,
  connection: &Connection,
  application: &RealtimeApplication,
) -> Result<(), OutboundSendError> {
  while let Some(result) = websocket_receiver.next().await {
    let frame = match result {
      Ok(message) => message,
      Err(error) => {
        tracing::error!(%error, "websocket read failed");

        return Ok(());
      }
    };

    match frame {
      WsMessage::Text(text) => {
        let message = match serde_json::from_str::<ProtocolMessage>(&text) {
          Ok(m) => m,
          Err(e) => {
            tracing::error!(%e, "invalid websocket message");

            let response = ProtocolMessage::nack(None);

            outbound_sender.try_enqueue_protocol_message(&response)?;

            continue
          }
        };

        let context = SocketContext {
          connection: &connection,
          sender: &outbound_sender,
          presence_hub: &application.presence_hub,
          channel_hub: &application.channel_hub,
        };

        let ProtocolOutcome {
          replies,
          disconnect
        } = handle_protocol_message(message, &context).await;

        for reply in replies {
          outbound_sender.try_enqueue_protocol_message(&reply)?;
        }

        if disconnect {
          return Ok(());
        }
      }
      WsMessage::Close(_) => {
        return Ok(());
      }
      _ => {}
    }
  }

  Ok(())
}

fn outbound_channel() -> (OutboundSender, Receiver<PreparedFrame>, watch::Receiver<bool>) {
  let (
    queue_sender,
    queue_receiver
  ) = mpsc::channel::<PreparedFrame>(OUTBOUND_QUEUE_CAPACITY);

  let (shutdown_sender, shutdown_receiver) = watch::channel(false);

  let sender = OutboundSender::new(queue_sender, shutdown_sender);

  (
    sender,
    queue_receiver,
    shutdown_receiver,
  )
}

fn send_connected(
  sender: &OutboundSender,
  connection: &Connection,
) -> Result<(), OutboundSendError> {
  let connected = ProtocolMessage::connected(&connection);

  sender.try_enqueue_protocol_message(&connected)
}

fn spawn_heartbeat(sender: &OutboundSender) -> JoinHandle<()> {
  tokio::spawn({
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
  })
}

fn spawn_writer(
  receiver: Receiver<PreparedFrame>,
  socket_sender: SplitSink<WebSocket, WsMessage>
) -> JoinHandle<()> {
  tokio::spawn(writer_loop(receiver, socket_sender))
}

async fn writer_loop(
  mut receiver: Receiver<PreparedFrame>,
  mut socket_sender: SplitSink<WebSocket, WsMessage>,
) {
  while let Some(frame) = receiver.recv().await {
    if let Err(error) = socket_sender
      .send(frame.into_websocket_message())
      .await
    {
      tracing::error!(%error, "websocket write failed");
      break;
    }
  }
}

async fn emit_connected(event_bus: &EventBus, connection: &Connection) {
  event_bus.emit(WebsocketConnected {
    connection_id: connection.id.as_str().to_string(),
  }).await
}

async fn cleanup_connection(connection: &Connection, application: &RealtimeApplication) {
  application.disconnect_connection(&connection.id).await;
}

async fn emit_disconnected(event_bus: &EventBus, connection: &Connection) {
  event_bus.emit(WebsocketDisconnected {
    connection_id: connection.id.as_str().to_string()
  }).await;
}