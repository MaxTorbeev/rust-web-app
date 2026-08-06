use std::sync::Arc;
use std::time::Duration;
use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::{SplitSink, SplitStream};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver};
use tokio::task::JoinHandle;
use event_bus::EventBus;
use crate::{Connection, OutboundSendError, OutboundSender, PreparedFrame, ProtocolMessage, ProtocolOutcome, RealtimeApplication, WebsocketConnected, WebsocketDisconnected};
use crate::protocol_handlers::{handle_protocol_message, SocketContext};

const OUTBOUND_QUEUE_CAPACITY: usize = 128;

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
    receiver
  ) = outbound_channel();

  let (
    websocket_sender,
    websocket_receiver,
  ) = socket.split();

  let mut writer_task = spawn_writer(receiver, websocket_sender);
  let mut heartbeat_task: Option<JoinHandle<()>> = None;

  // Запустить очередь
  let (
    result,
    writer_joined) = match send_connected(&outbound_sender, &connection).await {
    Ok(()) => {
      heartbeat_task = Some(
        spawn_heartbeat(&outbound_sender)
      );

      tokio::select! {
        protocol_result =    run_protocol_loop(
          websocket_receiver,
          &outbound_sender,
          &connection,
          &application
        ) => {
          // Reader завершился первым.
          // Writer ниже должен дописать очередь.
          (protocol_result, false)
        }

        writer_result = &mut writer_task => {
          // Writer завершился первым.
          // Отправлять сообщения в этот WebSocket больше невозможно.
          if let Err(error) = writer_result {
            tracing::error!(
              %error,
              "websocket writer task failed"
            );
          }

          (
            Err(OutboundSendError::QueueClosed),
            true,
          )
        }
      }
    }
    Err(error) => {
      // CONNECTED не удалось поставить в очередь.
      // Cleanup всё равно должен быть выполнен.
      (Err(error), false)
    }
  };

  // Больше не создаём heartbeat-сообщения.
  if let Some(heartbeat_task) = heartbeat_task {
    heartbeat_task.abort();
    let _ = heartbeat_task.await;
  }


  // Удаляем sender-клоны из ChannelHub
  // и очищаем presence.
  cleanup_connection(connection, application).await;

  // Уничтожаем последний локальный sender.
  // После этого Receiver закроется, когда обработает очередь.
  drop(outbound_sender);

  // Если writer не завершился внутри select!,
  // даём ему дописать оставшуюся очередь.
  if !writer_joined {
    if let Err(error) = writer_task.await {
      tracing::error!(
        %error,
        "websocket writer task failed during shutdown"
      );

      if result.is_ok() {
        return Err(OutboundSendError::QueueClosed);
      }
    }
  }

  result
}

async fn run_protocol_loop(
  mut websocket_receiver: SplitStream<WebSocket>,
  outbound_sender: &OutboundSender,
  connection: &Connection,
  application: &RealtimeApplication,
) -> Result<(), OutboundSendError> {
  while let Some(result) = websocket_receiver.next().await {
    let ws_message = match result {
      Ok(message) => message,
      Err(error) => {
        tracing::error!(%error, "websocket read failed");

        return Ok(());
      }
    };

    match ws_message {
      WsMessage::Text(text) => {
        let message = match serde_json::from_str::<ProtocolMessage>(&text) {
          Ok(m) => m,
          Err(e) => {
            tracing::error!(%e, "invalid websocket message");

            let response = ProtocolMessage::nack(None);

            outbound_sender.send_protocol(&response).await?;

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
          outbound_sender.send_protocol(&reply).await?;
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

fn outbound_channel() -> (OutboundSender, Receiver<PreparedFrame>) {
  let (
    queue_sender,
    receiver
  ) = mpsc::channel::<PreparedFrame>(OUTBOUND_QUEUE_CAPACITY);

  let sender = OutboundSender::new(queue_sender);

  (sender, receiver)
}

async fn send_connected(
  sender: &OutboundSender,
  connection: &Connection,
) -> Result<(), OutboundSendError> {
  let connected = ProtocolMessage::connected(&connection);

  sender.send_protocol(&connected).await
}

fn spawn_heartbeat(sender: &OutboundSender) -> JoinHandle<()> {
  tokio::spawn({
    let sender = sender.clone();

    async move {
      loop {
        tokio::time::sleep(Duration::from_millis(10_000)).await;

        let heartbeat = ProtocolMessage::heartbeat();

        if sender.send_protocol(&heartbeat).await.is_err() {
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