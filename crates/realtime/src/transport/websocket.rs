use std::sync::Arc;
use std::time::Duration;
use axum::extract::ws::{Message as WsMessage, Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::SplitSink;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use event_bus::EventBus;
use crate::{ChannelHub, Connection, ProtocolMessage, RealtimeApplication, WebsocketConnected, WebsocketDisconnected};
use crate::channel::presence_hub::PresenceHub;
use crate::protocol_handlers::{handle_protocol_message, SocketContext, ProtocolHandleResult};

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
    sender,
    mut receiver
  ) = mpsc::unbounded_channel::<ProtocolMessage>();

  let (mut socket_sender, mut socket_receiver) = socket.split();

  // writer loop
  let writer_task = tokio::spawn(async move {
    writer_loop(&mut receiver, &mut socket_sender).await;
  });

  if sender.send(ProtocolMessage::connected(&connection)).is_err() {
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

            if sender.send(response).is_err() {
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

        let ProtocolHandleResult {
          response,
          disconnect
        } = handle_protocol_message(message, &context).await;

        if let Some(response) = response {
          if sender.send(response).is_err() {
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

fn make_heartbeat_task(sender: &UnboundedSender<ProtocolMessage>) -> JoinHandle<()> {
  tokio::spawn({
    let sender = sender.clone();

    async move {
      loop {
        tokio::time::sleep(Duration::from_millis(10_000)).await;

        if sender.send(ProtocolMessage::heartbeat()).is_err() {
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

async fn writer_loop(receiver: &mut UnboundedReceiver<ProtocolMessage>, socket_sender: &mut SplitSink<WebSocket, Message>) {
  while let Some(message) = receiver.recv().await {
    let text = match serde_json::to_string(&message) {
      Ok(text) => text,
      Err(_e) => {
        tracing::error!(%_e, "websocket read failed");
        continue;
      }
    };

    if let Err(_e) = socket_sender.send(WsMessage::Text(text.into())).await {
      tracing::error!(%_e, "websocket read failed");
      break;
    }
  }
}