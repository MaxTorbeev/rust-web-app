use std::sync::Arc;
use std::time::Duration;
use axum::extract::ws::{Message as WsMessage, Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::SplitSink;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver};
use tokio::task::JoinHandle;
use event_bus::EventBus;
use crate::{ChannelHub, Connection, OutboundSender, PreparedFrame, ProtocolMessage, ProtocolOutcome, RealtimeApplication, WebsocketConnected, WebsocketDisconnected};
use crate::channel::presence_hub::PresenceHub;
use crate::protocol_handlers::{handle_protocol_message, SocketContext};

const OUTBOUND_QUEUE_CAPACITY: usize = 128;

pub async fn handle_socket(
  socket: WebSocket,
  connection: Connection,
  application: Arc<RealtimeApplication>,
  event_bus: Arc<EventBus>,
) {
  event_bus.emit(WebsocketConnected {
    connection_id: connection.id.as_str().to_string(),
  }).await;

  let (
    queue_sender,
    mut receiver
  ) = mpsc::channel::<PreparedFrame>(OUTBOUND_QUEUE_CAPACITY);

  let sender = OutboundSender::new(queue_sender);

  let (mut socket_sender, mut socket_receiver) = socket.split();

  // writer loop
  let writer_task = tokio::spawn(async move {
    writer_loop(&mut receiver, &mut socket_sender).await;
  });

  let connected = ProtocolMessage::connected(&connection);

  if sender.send_protocol(&connected).await.is_err() {
    tracing::error!("websocket outgoing queue closed");
    return;
  }

  let heartbeat_task = make_heartbeat_task(&sender);

  // reader loop
  while let Some(result) = socket_receiver.next().await {
    tracing::debug!(?result, "websocket message");

    let ws_message = match result {
      Ok(msg) => msg,
      Err(_e) => {
        tracing::error!(%_e, "websocket read failed");

        // пока можно отправлять напрямую через socket_sender,
        // но позже лучше через mpsc sender
        break;
      }
    };

    match ws_message {
      WsMessage::Text(text) => {
        let message = match serde_json::from_str::<ProtocolMessage>(&text) {
          Ok(m) => m,
          Err(e) => {
            tracing::error!(%e, "invalid websocket message");

            let response = ProtocolMessage::nack(None);

            if sender.send_protocol(&response).await.is_err() {
              tracing::error!("websocket outgoing queue closed");
              break;
            }

            continue
          }
        };

        let context = SocketContext {
          connection: &connection,
          sender: &sender,
          presence_hub: &application.presence_hub,
          channel_hub: &application.channel_hub,
        };

        let ProtocolOutcome {
          replies,
          disconnect
        } = handle_protocol_message(message, &context).await;

       for reply in replies {
          if sender.send_protocol(&reply).await.is_err() {
            tracing::error!("websocket outgoing queue closed");
            break;
          }
        }


        if disconnect {
          heartbeat_task.abort();
          break;
        }
      }
      _ => {}
    }
  }

  // Disconnection
  disconnect_socket(
    &connection,
    application.channel_hub.clone(),
    application.presence_hub.clone()
  ).await;

  // Read loop finished
  writer_task.abort();

  heartbeat_task.abort();

  event_bus.emit(WebsocketDisconnected {
    connection_id: connection.id.as_str().to_string()
  }).await;
}

fn make_heartbeat_task(sender: &OutboundSender) -> JoinHandle<()> {
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

async fn disconnect_socket(connection: &Connection, channel_hub: Arc<ChannelHub>, presence_hub: Arc<PresenceHub>) {
  let leaves = presence_hub.disconnect(&connection.id).await;

  channel_hub.disconnect(&connection.id).await;

  for (channel, presence) in leaves {
    channel_hub
      .broadcast(&channel, ProtocolMessage::presence(&channel, vec![presence]))
      .await;
  }
}

async fn writer_loop(receiver: &mut Receiver<PreparedFrame>, socket_sender: &mut SplitSink<WebSocket, Message>) {
  while let Some(message) = receiver.recv().await {
    if let Err(error) = socket_sender.send(message.into_websocket_message()).await {
      tracing::error!(%error, "websocket outgoing queue closed");
      break
    }
  }
}