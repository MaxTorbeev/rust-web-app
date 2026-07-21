use std::sync::Arc;
use axum::extract::ws::{Message as WsMessage, Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::SplitSink;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use event_bus::EventBus;
use crate::{ChannelHub, Connection, ProtocolAction, ProtocolMessage, WebsocketConnected, WebsocketDisconnected};

pub async fn handle_socket(
  socket: WebSocket,
  connection: Connection,
  channel_hub: Arc<ChannelHub>,
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

        let response = match message.action {
          ProtocolAction::Connect => ProtocolMessage::connected(&connection),

          ProtocolAction::Attach => match message.channel.as_deref() {
            Some(channel) => {
              channel_hub.attach(channel, connection.id.clone(), sender.clone()).await;

              ProtocolMessage::attached(&message)
            },
            None => ProtocolMessage::nack(message.msg_serial)
          },

          ProtocolAction::Presence => ProtocolMessage::ack(&message),
          ProtocolAction::Message => match message.channel.as_deref() {
            Some(channel) => {
              channel_hub
                .broadcast(channel, message.clone())
                .await;

              ProtocolMessage::ack(&message)
            },
            None => ProtocolMessage::nack(message.msg_serial)
          },
          ProtocolAction::Heartbeat => ProtocolMessage::heartbeat(),
          _ => continue,
        };

        if sender.send(response).is_err() {
          tracing::error!("websocket outgoing queue closed");
          break;
        }
      }
      _ => {}
    }
  }

  // Disconnection
  channel_hub.disconnect(&connection.id).await;

  // Read loop finished
  writer_task.abort();

  event_bus.emit(WebsocketDisconnected {
    connection_id: connection.id.as_str().to_string()
  }).await;
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